use futures::{stream::FuturesUnordered, StreamExt};
use serde_json::Value;
use std::fs;

#[derive(Debug, Default)]
pub struct Package {
    pub name: String,
    pub exports: Option<String>,
    pub package_type: Option<String>,
}

impl Package {
    pub fn has_values(&self) -> bool {
        self.exports.is_some() || self.package_type.is_some()
    }
}

pub async fn generate_packages(short: bool) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .user_agent("esm-checker/0.1.0 (+https://github.com/lannonbr/esm-checker)")
        .build()
        .unwrap();

    let mut initial_package_list: Vec<String> = fs::read_to_string("packages.txt")
        .unwrap()
        .split_whitespace()
        .into_iter()
        .map(|c| c.to_owned())
        .collect();

    if short {
        initial_package_list.truncate(100);
    }

    let mut requests = FuturesUnordered::new();

    for package in initial_package_list {
        let client = client.clone();
        requests.push(tokio::spawn(async move {
            let url = format!("https://unpkg.com/{}@latest/package.json", package);
            client.get(url).send().await.unwrap().text().await.unwrap()
        }));
    }

    let mut pkgs: Vec<Package> = vec![];

    while let Some(unpkg_resp) = requests.next().await {
        let package_json_str = unpkg_resp.unwrap();
        let package_json: Value = match serde_json::from_str(&package_json_str) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("{}", e);
                println!("Str: {}", package_json_str);
                continue;
            }
        };
        let name_opt = package_json.get("name");

        let mut new_package = Package::default();

        let name = name_opt.unwrap().as_str().unwrap();
        new_package.name = name.to_string();

        if let Some(package_type) = package_json.get("type") {
            new_package.package_type = Some(package_type.as_str().unwrap().to_string());
        }
        if let Some(exports) = package_json.get("exports") {
            new_package.exports = Some(exports.to_string());
        }

        pkgs.push(new_package);
    }

    Ok(pkgs)
}
