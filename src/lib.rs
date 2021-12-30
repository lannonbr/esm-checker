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

    let mut pkgs: Vec<Package> = vec![];

    for package in initial_package_list {
        let client = client.clone();
        requests.push(tokio::spawn(async move {
            let url = format!("https://unpkg.com/{}@latest/package.json", package);
            client.get(url).send().await.unwrap().text().await.unwrap()
        }));

        if requests.len() > 30 {
            let package_json_str = requests.next().await.unwrap().unwrap();

            let package = generate_pkg(package_json_str);

            if package.is_some() {
                pkgs.push(package.unwrap());
            }
        }
    }

    while let Some(unpkg_resp) = requests.next().await {
        let package_json_str = unpkg_resp.unwrap();

        let package = generate_pkg(package_json_str);

        if package.is_some() {
            pkgs.push(package.unwrap());
        }
    }

    Ok(pkgs)
}

fn generate_pkg(json_str: String) -> Option<Package> {
    let package_json: Value = match serde_json::from_str(&json_str) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{}", e);
            println!("Str: {}", json_str);
            return None;
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

    Some(new_package)
}
