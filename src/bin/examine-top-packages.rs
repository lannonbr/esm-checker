use aws_sdk_dynamodb::model::AttributeValue;
use aws_types::region::Region;
use futures::{stream::FuturesUnordered, StreamExt};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, process::exit};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "examine-top-packages")]
struct Opt {
    /// Output packages with any of `type` or `exports` fields in them as TSV
    #[structopt(long)]
    tsv: bool,

    /// Output stats on the packages
    #[structopt(long)]
    stats: bool,

    /// Use this flag to reduce the requests to the first 100
    #[structopt(long)]
    short: bool,
}

#[derive(Debug, Default)]
struct Package {
    name: String,
    exports: Option<String>,
    package_type: Option<String>,
}

impl Package {
    fn has_values(&self) -> bool {
        self.exports.is_some() || self.package_type.is_some()
    }
}

async fn generate_packages(short: bool) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
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
            Err(_) => {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = std::env::var("DYNAMO_STATS_TABLE_NAME")
        .expect("There should be a table name defined as an environment variable");
    let aws_region = std::env::var("AWS_REGION").ok().map(Region::new).unwrap();

    let config = aws_config::from_env().region(aws_region).load().await;

    let dynamo_client = aws_sdk_dynamodb::Client::new(&config);

    let args = Opt::from_args();

    let mut packages = generate_packages(args.short).await?;

    if !args.tsv && !args.stats {
        println!("Please add one of the following flags `--tsv` or `--stats`, view `--help` for more information.");
        exit(1);
    }
    let total_packages = packages.len();

    packages = packages.into_iter().filter(|p| p.has_values()).collect();

    let type_module_count = packages
        .iter()
        .filter(|&p| p.package_type == Some(String::from("module")))
        .count();

    let require_count = packages
        .iter()
        .filter(|&p| p.exports.is_some())
        .filter(|&p| p.exports.as_ref().unwrap().contains("require"))
        .count();

    let esm_only = packages
        .iter()
        .filter(|&p| p.exports.is_some())
        .filter(|&p| !p.exports.as_ref().unwrap().contains("require"))
        .count();

    if args.tsv {
        println!("name\tpackage_type\texports");
        for pkg in packages {
            println!(
                "{}\t{:?}\t{:?}",
                pkg.name,
                pkg.package_type.unwrap_or_default(),
                pkg.exports.unwrap_or_default()
            );
        }
    }

    if args.stats {
        let unix_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Should have a unix timestamp")
            .as_millis();

        println!("Total packages: {}", total_packages);
        println!(
            "Packages with a `type: module` field: {}",
            type_module_count
        );
        println!("Packages with a `exports.require` field: {}", require_count);
        println!(
            "Packages without an explicit `exports.require` that may be ESM only: {}",
            esm_only
        );

        let request = dynamo_client
            .put_item()
            .table_name(table_name)
            .item("timestamp", AttributeValue::N(unix_timestamp.to_string()))
            .item(
                "total_packages",
                AttributeValue::N(total_packages.to_string()),
            )
            .item(
                "type_module",
                AttributeValue::N(type_module_count.to_string()),
            )
            .item(
                "exports_require",
                AttributeValue::N(require_count.to_string()),
            )
            .item(
                "exports_no_require",
                AttributeValue::N(esm_only.to_string()),
            );

        request.send().await?;
    }

    Ok(())
}
