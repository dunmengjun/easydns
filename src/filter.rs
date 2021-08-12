use crate::config::Config;
use crate::system::{Result, FileNotFoundError};
use regex::Regex;
use std::collections::HashSet;
use std::process::Stdio;
use tokio::fs::File;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, BufReader};
use tokio::process::Command;

const GET_DOMAIN_REGEX: &str =
    "address /([a-zA-Z0-9][-a-zA-Z0-9]{0,62}(?:\\.[a-zA-Z0-9][-a-zA-Z0-9]{0,62})+)/([#|d])";

#[derive(PartialEq, Eq, Hash, Debug)]
struct FilterItem {
    domain: String,
    group: String,
}

impl From<String> for FilterItem {
    fn from(domain: String) -> Self {
        FilterItem {
            domain,
            group: "#".into(),
        }
    }
}

pub struct Filter {
    set: HashSet<FilterItem>,
}

impl Filter {
    pub async fn from(config: &Config) -> Self {
        let set = read_resources_to_filter(&config.filters).await;
        debug!("filter init done, set len = {}", set.len());
        Filter { set }
    }

    pub fn contain(&self, domain: String) -> bool {
        //拆分多级域名
        let split = domain.split(".");
        let vec: Vec<&str> = split.collect();
        for i in (0..vec.len()).rev() {
            let mut string = String::new();
            for j in i..vec.len() {
                string.push_str(vec[j]);
                string.push_str(".");
            }
            string.remove(string.len() - 1);
            if self.set.contains(&string.into()) {
                return true;
            }
        }
        false
    }
}

async fn read_resources_to_filter(paths: &Vec<String>) -> HashSet<FilterItem> {
    let mut set = HashSet::new();
    for path in paths {
        let result = read_resource_to_filter(&path).await;
        match result {
            Ok(temp) => {
                for f in temp {
                    if f.group == "#" {
                        set.insert(f);
                    } else {
                        set.remove(&f.domain.into());
                    }
                }
            }
            Err(e) => {
                error!("{:?}", e);
            }
        };
    }
    set
}

async fn read_resource_to_filter(path: &str) -> Result<HashSet<FilterItem>> {
    if path.starts_with("http") {
        read_url_to_filter(path).await
    } else {
        read_file_to_filter(path).await
    }
}

async fn read_url_to_filter(url: &str) -> Result<HashSet<FilterItem>> {
    let mut child = Command::new("curl")
        .arg("-k")
        .arg("-s")
        .arg(url)
        .stdout(Stdio::piped())
        .spawn()?;
    let reader = BufReader::new(child.stdout.take().unwrap());
    tokio::spawn(async move {
        let status = child
            .wait()
            .await
            .expect("filter curl process encountered an error");
        debug!("filter curl status was: {}", status);
    });
    read_to_filter(reader).await
}

async fn read_file_to_filter(file_path: &str) -> Result<HashSet<FilterItem>> {
    let file = File::open(file_path).await.map_err(|e| {
        FileNotFoundError {
            path: String::from(file_path),
            supper: Box::new(e),
        }
    })?;
    let reader = BufReader::new(file);
    read_to_filter(reader).await
}

async fn read_to_filter(
    mut reader: impl AsyncBufRead + std::marker::Unpin,
) -> Result<HashSet<FilterItem>> {
    let mut buffer = String::new();
    let line_regex = Regex::new(GET_DOMAIN_REGEX).unwrap();
    let mut set = HashSet::new();
    while reader.read_line(&mut buffer).await? > 0 {
        match handle_one_line(&line_regex, &buffer) {
            None => {}
            Some(item) => {
                set.insert(item);
            }
        }
        buffer.clear();
    }
    Ok(set)
}

fn handle_one_line(regex: &Regex, line: &String) -> Option<FilterItem> {
    if line.starts_with("#") {
        return None;
    }
    regex
        .captures(line)
        .and_then(|cap| match cap.get(1).map(|l| String::from(l.as_str())) {
            Some(domain) => {
                if let Some(group) = cap.get(2).map(|f| String::from(f.as_str())) {
                    Some(FilterItem { domain, group })
                } else {
                    None
                }
            }
            None => None,
        })
}

#[cfg(test)]
mod tests {
    use crate::filter::{
        handle_one_line, read_resource_to_filter, read_resources_to_filter, FilterItem,
        GET_DOMAIN_REGEX,
    };
    use crate::system::Result;
    use regex::Regex;
    use std::collections::HashSet;

    #[test]
    fn test_handle_one_line() {
        let line_regex = Regex::new(GET_DOMAIN_REGEX).unwrap();
        let x = String::from("address /kwcscdn.000dn.com/#");

        let result = handle_one_line(&line_regex, &x);

        assert_eq!(result, Some(String::from("kwcscdn.000dn.com").into()));
    }

    #[tokio::test]
    async fn test_read_file_to_filter() -> Result<()> {
        let filter = read_resource_to_filter(
            "./tests/resources/test_filter.txt").await?;

        let mut expected: HashSet<FilterItem> = HashSet::new();
        expected.insert(String::from("00-gov.cn").into());
        expected.insert(String::from("kwcdn.000dn.com").into());
        assert_eq!(expected, filter);
        Ok(())
    }

    #[tokio::test]
    async fn test_read_url_to_filter() -> Result<()> {
        let filter = read_resource_to_filter(
            "https://raw.githubusercontent.com/dunmengjun\
            /SmartDNS-GFWList/master/test_url_filter.txt",
        ).await?;

        let mut expected: HashSet<FilterItem> = HashSet::new();
        expected.insert(String::from("00-gov.cn").into());
        expected.insert(String::from("kwcdn.000dn.com").into());
        assert_eq!(expected, filter);
        Ok(())
    }

    #[tokio::test]
    async fn test_filter_item_overcast() -> Result<()> {
        let filters: Vec<String> = vec!["./tests/resources/test_filter.txt".into(),
                                        "./tests/resources/covercast_filter.txt".into()];

        let result = read_resources_to_filter(&filters).await;

        let mut expected: HashSet<FilterItem> = HashSet::new();
        expected.insert(String::from("00-gov.cn").into());
        assert_eq!(expected, result);
        Ok(())
    }

    #[tokio::test]
    async fn test_filter_path_empty() -> Result<()> {
        let filters: Vec<String> = vec![];

        let result = read_resources_to_filter(&filters).await;

        assert!(result.is_empty());
        Ok(())
    }
}
