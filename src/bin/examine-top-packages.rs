use aws_sdk_dynamodb::model::AttributeValue;
use aws_types::region::Region;
use esm_checker::generate_packages;
use std::process::exit;
use std::time::{SystemTime, UNIX_EPOCH};
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

    /// Publish stats to dynamo
    #[structopt(long)]
    dynamo: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

        if args.dynamo {
            let table_name = std::env::var("DYNAMO_STATS_TABLE_NAME")
                .expect("There should be a table name defined as an environment variable");
            let aws_region = std::env::var("AWS_REGION").ok().map(Region::new).unwrap();

            let config = aws_config::from_env().region(aws_region).load().await;

            let dynamo_client = aws_sdk_dynamodb::Client::new(&config);
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
    }

    Ok(())
}
