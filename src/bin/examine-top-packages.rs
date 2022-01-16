use aws_sdk_dynamodb::model::AttributeValue;
use aws_types::region::Region;
use chrono::Utc;
use clap::StructOpt;
use esm_checker::{generate_packages, AuditEntry, Package};
use std::collections::HashMap;

#[derive(StructOpt, Debug)]
#[structopt(name = "examine-top-packages")]
struct Opt {
    /// Use this flag to reduce the requests to the first 100
    #[structopt(long)]
    short: bool,

    /// Publish stats to dynamo
    #[structopt(long)]
    dynamo: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Opt::parse();

    let mut packages = generate_packages(args.short).await?;
    let all_packages = packages.clone();

    let total_packages = packages.len();

    packages = packages.into_iter().filter(|p| p.has_values()).collect();

    let type_module_count = packages.iter().filter(|&p| p.type_module).count();

    let require_count = packages.iter().filter(|&p| p.exports_require).count();

    let exports_no_require = packages.iter().filter(|&p| p.exports_no_require).count();

    let date_time = Utc::now();
    let date = date_time.format("%F").to_string();
    let month_year_date = date_time.format("%Y-%m").to_string();

    println!("Total packages: {}", total_packages);
    println!(
        "Packages with a `type: module` field: {}",
        type_module_count
    );
    println!("Packages with a `exports.require` field: {}", require_count);
    println!(
        "Packages without an explicit `exports.require` that may be ESM only: {}",
        exports_no_require
    );

    if args.dynamo {
        let stats_table_name = std::env::var("DYNAMO_STATS_TABLE_NAME")
            .expect("There should be a stats table name defined as an environment variable");
        let package_table_name = std::env::var("DYNAMO_PACKAGE_TABLE_NAME")
            .expect("There should be a pacakge table name defined as an environment variable");
        let audit_table_name = std::env::var("DYNAMO_AUDIT_TABLE_NAME")
            .expect("There should be an audit table name defined as an environment variable");

        let aws_region = std::env::var("AWS_REGION").ok().map(Region::new).unwrap();
        let config = aws_config::from_env().region(aws_region).load().await;
        let dynamo_client = aws_sdk_dynamodb::Client::new(&config);

        put_stats(
            &dynamo_client,
            stats_table_name,
            &month_year_date,
            &date,
            StatsObject {
                total_packages,
                type_module_count,
                require_count,
                exports_no_require,
            },
        )
        .await;

        let mut new_packages: Vec<Package> = Vec::new();
        let package_table_map = grab_remote_packages(&dynamo_client, &package_table_name).await;

        // Diff packages with their state in dynamo and update entries & create audit points if there are changes
        for pkg in all_packages {
            if package_table_map.contains_key(&pkg.name) {
                let old_pkg = package_table_map.get(&pkg.name).unwrap();
                let (should_update, mut local_audits) = diff_packages(old_pkg, &pkg, &date);

                if should_update {
                    update_package(&dynamo_client, &package_table_name, &pkg).await;
                }
                while local_audits.len() > 0 {
                    let audit = local_audits.pop().unwrap();
                    put_audit(&dynamo_client, &audit_table_name, audit).await;
                }
            } else {
                new_packages.push(pkg);
            }
        }

        // Create entries for brand new packages in package table
        for pkg in new_packages {
            put_package(&dynamo_client, &package_table_name, pkg).await;
        }
    }

    Ok(())
}

struct StatsObject {
    total_packages: usize,
    type_module_count: usize,
    require_count: usize,
    exports_no_require: usize,
}

async fn put_stats(
    dynamo_client: &aws_sdk_dynamodb::Client,
    stats_table_name: String,
    month_year_date: &String,
    date: &String,
    stats: StatsObject,
) {
    let request = dynamo_client
        .put_item()
        .table_name(stats_table_name)
        .item("year_month", AttributeValue::S(month_year_date.clone()))
        .item("timestamp", AttributeValue::S(date.clone()))
        .item(
            "total_packages",
            AttributeValue::N(stats.total_packages.to_string()),
        )
        .item(
            "type_module",
            AttributeValue::N(stats.type_module_count.to_string()),
        )
        .item(
            "exports_require",
            AttributeValue::N(stats.require_count.to_string()),
        )
        .item(
            "exports_no_require",
            AttributeValue::N(stats.exports_no_require.to_string()),
        );
    let r = request.send().await;
    if r.is_err() {
        eprintln!("err: {}", r.unwrap_err());
    }
}

async fn grab_remote_packages(
    dynamo_client: &aws_sdk_dynamodb::Client,
    package_table_name: &String,
) -> HashMap<String, Package> {
    let scan_result = dynamo_client
        .scan()
        .table_name(package_table_name.clone())
        .send()
        .await
        .unwrap();
    let items = scan_result.items.unwrap();

    let mut package_table_map: HashMap<String, Package> = HashMap::new();

    for pkg_hash in items {
        let pkg = Package::from(pkg_hash);
        package_table_map.insert(pkg.name.clone(), pkg);
    }
    package_table_map
}

async fn put_package(
    dynamo_client: &aws_sdk_dynamodb::Client,
    package_table_name: &String,
    pkg: Package,
) {
    let r = dynamo_client
        .put_item()
        .table_name(package_table_name.clone())
        .item("package_name", AttributeValue::S(pkg.name))
        .item("exports_require", AttributeValue::Bool(pkg.exports_require))
        .item(
            "exports_no_require",
            AttributeValue::Bool(pkg.exports_no_require),
        )
        .item("type_module", AttributeValue::Bool(pkg.type_module))
        .send()
        .await;
    if r.is_err() {
        println!("err: {}", r.unwrap_err());
    }
}

async fn update_package(
    dynamo_client: &aws_sdk_dynamodb::Client,
    package_table_name: &String,
    pkg: &Package,
) {
    let r = dynamo_client
        .update_item()
        .table_name(package_table_name.clone())
        .key("package_name", AttributeValue::S(pkg.name.clone()))
        .update_expression("SET exports_require=:exports_require, exports_no_require=:exports_no_require, type_module=:type_module")
        .expression_attribute_values(":exports_require", AttributeValue::Bool(pkg.exports_require))
        .expression_attribute_values(":exports_no_require", AttributeValue::Bool(pkg.exports_no_require))
        .expression_attribute_values(":type_module", AttributeValue::Bool(pkg.type_module))
        .send()
        .await;
    if r.is_err() {
        println!("err: {}", r.unwrap_err());
    }
}

async fn put_audit(
    dynamo_client: &aws_sdk_dynamodb::Client,
    audit_table_name: &String,
    audit: AuditEntry,
) {
    let uuid = uuid::Uuid::new_v4();

    let r = dynamo_client
        .put_item()
        .table_name(audit_table_name.clone())
        .item("timestamp", AttributeValue::S(audit.timestamp))
        .item(
            "package_name_id",
            AttributeValue::S(format!(
                "{}{}",
                audit.package_name.clone(),
                &uuid.to_simple().to_string()
            )),
        )
        .item("package_name", AttributeValue::S(audit.package_name))
        .item("change", AttributeValue::S(audit.change))
        .item("old_value", AttributeValue::Bool(audit.old_value))
        .item("new_value", AttributeValue::Bool(audit.new_value))
        .send()
        .await;
    if r.is_err() {
        println!("err: {}", r.unwrap_err());
    }
}

fn diff_packages(old_pkg: &Package, pkg: &Package, date: &String) -> (bool, Vec<AuditEntry>) {
    let mut should_update = false;
    let mut audits: Vec<AuditEntry> = Vec::new();

    if old_pkg.type_module != pkg.type_module {
        println!(
            "change in type module {} to {}",
            old_pkg.type_module, pkg.type_module
        );

        should_update = true;

        let audit_entry = AuditEntry {
            package_name: pkg.name.clone(),
            timestamp: date.clone(),
            change: String::from("type_module"),
            old_value: old_pkg.type_module,
            new_value: pkg.type_module,
        };

        audits.push(audit_entry);
    }
    if old_pkg.exports_require != pkg.exports_require {
        println!(
            "change in exports.require {} to {}",
            old_pkg.exports_require, pkg.exports_require
        );

        should_update = true;

        let audit_entry = AuditEntry {
            package_name: pkg.name.clone(),
            timestamp: date.clone(),
            change: String::from("exports_require"),
            old_value: old_pkg.exports_require,
            new_value: pkg.exports_require,
        };
        audits.push(audit_entry);
    }
    if old_pkg.exports_no_require != pkg.exports_no_require {
        println!(
            "change in no exports.require {} to {}",
            old_pkg.exports_no_require, pkg.exports_no_require
        );

        should_update = true;

        let audit_entry = AuditEntry {
            package_name: pkg.name.clone(),
            timestamp: date.clone(),
            change: String::from("exports_no_require"),
            old_value: old_pkg.exports_no_require,
            new_value: pkg.exports_no_require,
        };
        audits.push(audit_entry);
    }

    (should_update, audits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_type_module() {
        let old_pkg = Package {
            name: String::from("test-package"),
            exports_no_require: false,
            exports_require: false,
            type_module: false,
        };
        let new_pkg = Package {
            name: String::from("test-package"),
            exports_no_require: false,
            exports_require: false,
            type_module: true,
        };

        let result = diff_packages(&old_pkg, &new_pkg, &String::from("2022-01-01"));

        assert_eq!(result.0, true);
        assert_eq!(result.1.len(), 1);
        assert_eq!(result.1[0].change, String::from("type_module"));
    }

    #[test]
    fn test_multiple() {
        let old_pkg = Package {
            name: String::from("test-package"),
            exports_no_require: false,
            exports_require: false,
            type_module: false,
        };
        let new_pkg = Package {
            name: String::from("test-package"),
            exports_no_require: false,
            exports_require: true,
            type_module: true,
        };

        let result = diff_packages(&old_pkg, &new_pkg, &String::from("2022-01-01"));

        assert_eq!(result.0, true);
        assert_eq!(result.1.len(), 2);
    }

    #[test]
    fn test_no_change() {
        let old_pkg = Package {
            name: String::from("test-package"),
            exports_no_require: false,
            exports_require: false,
            type_module: false,
        };
        let new_pkg = Package {
            name: String::from("test-package"),
            exports_no_require: false,
            exports_require: false,
            type_module: false,
        };

        let result = diff_packages(&old_pkg, &new_pkg, &String::from("2022-01-01"));

        assert_eq!(result.0, false);
        assert_eq!(result.1.len(), 0);
    }
}
