use std::{collections::HashMap, fs};

fn main() {
    let registry_str = fs::read_to_string("registry.txt").unwrap();

    let mut hash: HashMap<&str, u32> = HashMap::new();

    let packages = registry_str.split(",");

    let prefix_size = 4;

    for package in packages {
        if package.len() >= prefix_size {
            let prefix = &package[0..prefix_size];
            hash.entry(prefix).and_modify(|e| *e += 1).or_insert(1);
        }
    }

    let mut prefixes: Vec<(&str, u32)> = hash.iter().map(|(&k, &v)| (k, v)).collect();

    prefixes.sort_by(|a, b| b.1.cmp(&a.1));

    println!("Top 10 prefixes with length {}", prefix_size);
    for i in 0..10 {
        let (prefix, prefix_count) = prefixes[i];
        println!("prefixes[{}]: {}", prefix, prefix_count);
    }
}
