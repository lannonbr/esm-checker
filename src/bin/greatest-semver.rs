use aws_sdk_dynamodb::model::AttributeValue;
use aws_types::region::Region;
use futures::{stream::FuturesOrdered, StreamExt};
use semver::Version;
use serde_json::Value;

/// The following is a one-off CLI script that adds a "greatest_semver" attribute to every package in the ESM-Checker Package table.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let package_table_name = std::env::var("DYNAMO_PACKAGE_TABLE_NAME")
        .expect("There should be a pacakge table name defined as an environment variable");

    let aws_region = std::env::var("AWS_REGION").ok().map(Region::new).unwrap();
    let config = aws_config::from_env().region(aws_region).load().await;
    let dynamo_client = aws_sdk_dynamodb::Client::new(&config);

    let scan_result = dynamo_client
        .scan()
        .table_name(&package_table_name)
        .projection_expression("package_name")
        .send()
        .await
        .unwrap();

    let items = scan_result.items.unwrap();

    let mut pkg_names: Vec<String> = Vec::new();

    for scan_item in items {
        let pkg_name = scan_item["package_name"].as_s().unwrap().to_owned();
        pkg_names.push(pkg_name);
    }

    let mut requests = FuturesOrdered::new();

    let client = reqwest::Client::builder()
        .user_agent("esm-checker/0.3.0 (+https://github.com/lannonbr/esm-checker)")
        .build()
        .unwrap();

    for pkg_name in pkg_names {
        let client = client.clone();
        let name = pkg_name.clone();
        requests.push(tokio::spawn(async move {
            let url = format!("https://registry.npmjs.com/{}", &name);
            (
                name,
                client.get(url).send().await.unwrap().text().await.unwrap(),
            )
        }));

        if requests.len() > 20 {
            let (name, registry_txt) = requests.next().await.unwrap().unwrap();

            let registry_json: Value = serde_json::from_str(&registry_txt).unwrap();

            let greatest_stable_semver = find_semver(&registry_json, &name);

            dynamo_client
                .update_item()
                .table_name(&package_table_name)
                .key("package_name", AttributeValue::S(name))
                .update_expression("SET greatest_semver = :gs")
                .expression_attribute_values(":gs", AttributeValue::S(greatest_stable_semver))
                .send()
                .await
                .expect("Should have successfully updated item");
        }
    }

    while let Some(resp) = requests.next().await {
        let (name, registry_txt) = resp.unwrap();

        let registry_json: Value = serde_json::from_str(&registry_txt).unwrap();

        let greatest_stable_semver = find_semver(&registry_json, &name);

        dynamo_client
            .update_item()
            .table_name(&package_table_name)
            .key("package_name", AttributeValue::S(name))
            .update_expression("SET greatest_semver = :gs")
            .expression_attribute_values(":gs", AttributeValue::S(greatest_stable_semver))
            .send()
            .await
            .expect("Should have successfully updated item");
    }

    Ok(())
}

fn find_semver(val: &Value, pkg_name: &String) -> String {
    let versions = match val["versions"].as_object() {
        Some(s) => s,
        None => {
            println!("{}: {}", pkg_name, val.to_string());
            panic!("Failed to find `versions` field within JSON for package");
        }
    };

    versions
        .keys()
        .filter(|v| {
            let version = Version::parse(v).unwrap();

            version.pre.is_empty()
        })
        .max_by(|a, b| Version::parse(a).unwrap().cmp(&Version::parse(b).unwrap()))
        .unwrap()
        .to_owned()
}
