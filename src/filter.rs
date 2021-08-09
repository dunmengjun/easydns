use std::collections::HashSet;
use tokio::fs::File;
use crate::system::Result;
use tokio::io::{BufReader, AsyncBufReadExt, AsyncBufRead};
use regex::Regex;
use std::process::Stdio;
use crate::config::Config;
use tokio::sync::OnceCell;
use tokio::process::Command;

const GET_DOMAIN_REGEX: &str = "address /([a-zA-Z0-9][-a-zA-Z0-9]{0,62}(\\.[a-zA-Z0-9][-a-zA-Z0-9]{0,62})+)/#";

struct FilterContext {
    set: HashSet<String>,
}

impl FilterContext {
    async fn from(config: &Config) -> Self {
        let mut set = HashSet::new();
        for path in &config.filters {
            let result = if path.starts_with("http") {
                read_url_to_filter(path).await
            } else {
                read_file_to_filter(path).await
            };
            let temp_set = match result {
                Ok(set) => set,
                Err(e) => {
                    error!("error occur in filter set init: {:?}", e);
                    HashSet::new()
                }
            };
            set.extend(temp_set)
        }
        debug!("filter init done, set len = {}", set.len());
        FilterContext {
            set
        }
    }
}

static FILTER_CONTEXT: OnceCell<FilterContext> = OnceCell::const_new();

pub async fn init_context(config: &Config) -> Result<()> {
    let context = FilterContext::from(config).await;
    match FILTER_CONTEXT.set(context) {
        Ok(_) => {}
        Err(e) => {
            panic!("{}", e);
        }
    }
    Ok(())
}

fn filter_set() -> &'static HashSet<String> {
    &FILTER_CONTEXT.get().unwrap().set
}

async fn read_url_to_filter(url: &str) -> Result<HashSet<String>> {
    let mut child = Command::new("curl")
        .arg("-k")
        .arg("-s")
        .arg(url)
        .stdout(Stdio::piped())
        .spawn()?;
    let reader = BufReader::new(child.stdout.take().unwrap());
    tokio::spawn(async move {
        let status = child.wait().await
            .expect("filter curl process encountered an error");
        debug!("filter curl status was: {}", status);
    });
    read_to_filter(reader).await
}

async fn read_file_to_filter(file_path: &str) -> Result<HashSet<String>> {
    let file = File::open(file_path).await?;
    let reader = BufReader::new(file);
    read_to_filter(reader).await
}

async fn read_to_filter(mut reader: impl AsyncBufRead + std::marker::Unpin) -> Result<HashSet<String>> {
    let mut buffer = String::new();
    let line_regex = Regex::new(GET_DOMAIN_REGEX).unwrap();
    let mut set = HashSet::new();
    while reader.read_line(&mut buffer).await? > 0 {
        match handle_one_line(&line_regex, &buffer) {
            None => {}
            Some(domain) => {
                set.insert(domain);
            }
        }
        buffer.clear();
    }
    Ok(set)
}

fn handle_one_line(regex: &Regex, line: &String) -> Option<String> {
    if line.starts_with("#") {
        return None;
    }
    regex.captures(line).and_then(|cap|
        cap.get(1).map(|l| String::from(l.as_str()))
    )
}

pub fn contain(domain: String) -> bool {
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
        if filter_set().contains(&string) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use regex::Regex;
    use crate::filter::{handle_one_line, read_file_to_filter, GET_DOMAIN_REGEX, read_url_to_filter};
    use crate::system::Result;
    use std::collections::HashSet;

    #[test]
    fn test_handle_one_line() {
        let line_regex = Regex::new(GET_DOMAIN_REGEX).unwrap();
        let x = String::from("address /kwcscdn.000dn.com/#");
        let result = handle_one_line(&line_regex, &x);

        assert_eq!(result, Some(String::from("kwcscdn.000dn.com")));
    }

    #[tokio::test]
    async fn test_read_file_to_filter() -> Result<()> {
        let filter = read_file_to_filter("test_filter.txt").await?;
        let mut expected = HashSet::new();
        expected.insert(String::from("00-gov.cn"));
        expected.insert(String::from("kwcdn.000dn.com"));
        assert_eq!(expected, filter);
        Ok(())
    }

    #[tokio::test]
    async fn test_read_url_to_filter() -> Result<()> {
        let filter = read_url_to_filter(
            "https://raw.githubusercontent.com/dunmengjun\
            /SmartDNS-GFWList/master/test_url_filter.txt").await?;
        let mut expected = HashSet::new();
        expected.insert(String::from("00-gov.cn"));
        expected.insert(String::from("kwcdn.000dn.com"));
        assert_eq!(expected, filter);
        Ok(())
    }
}