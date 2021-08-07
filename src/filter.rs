use std::collections::HashSet;
use once_cell::sync::Lazy;
use tokio::task::block_in_place;
use tokio::runtime::Handle;

pub static FILTER_SET: Lazy<HashSet<String>> = Lazy::new(|| {
    block_in_place(move || {
        Handle::current().block_on(async move {
            let mut set = HashSet::new();
            set.insert("www.google.com".to_string());
            set
        })
    })
});

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
        if FILTER_SET.contains(&string) {
            return true;
        }
    }
    false
}