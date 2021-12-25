use std::{collections::HashMap, fs};

fn main() {
    let s = fs::read_to_string("registry.txt").unwrap();

    let mut hash: HashMap<&str, u32> = HashMap::new();

    let it = s.split(",");

    let prefix_size = 4;

    for i in it {
        if i.len() >= prefix_size {
            let key = &i[0..prefix_size];
            hash.entry(key).and_modify(|e| *e += 1).or_insert(1);
        }
    }

    let mut prefixes: Vec<(&str, u32)> = hash.iter().map(|(&k, &v)| (k, v)).collect();

    prefixes.sort_by(|a, b| a.1.cmp(&b.1));
    prefixes.reverse();

    println!("Top 10 prefixes with length {}", prefix_size);
    for i in 0..10 {
        println!("prefixes[{}]: {}", prefixes[i].0, prefixes[i].1);
    }
}
