use aws_sdk_dynamodb::model::AttributeValue;
use aws_types::region::Region;
use futures::{stream::FuturesOrdered, StreamExt};
use semver::Version;
use serde_json::Value;

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
            client.get(url).send().await.unwrap().text().await.unwrap()
        }));

        if requests.len() > 20 {
            let registry_txt = requests.next().await.unwrap().unwrap();

            let registry_json: Value = serde_json::from_str(&registry_txt).unwrap();

            let greatest_stable_semver = find_semver(&registry_json);

            dbg!(&pkg_name, &greatest_stable_semver);

            let resp = dynamo_client
                .update_item()
                .table_name(&package_table_name)
                .key("package_name", AttributeValue::S(pkg_name))
                .update_expression("SET greatest_semver = :gs")
                .expression_attribute_values(":gs", AttributeValue::S(greatest_stable_semver))
                .send()
                .await;
        }
    }

    while let Some(resp) = requests.next().await {
        let registry_txt = resp.unwrap();

        let registry_json: Value = serde_json::from_str(&registry_txt).unwrap();
        let pkg_name = registry_json["name"].to_string();

        let greatest_stable_semver = find_semver(&registry_json);

        dbg!(&pkg_name, &greatest_stable_semver);

        let resp = dynamo_client
            .update_item()
            .table_name(&package_table_name)
            .key("package_name", AttributeValue::S(pkg_name))
            .update_expression("SET greatest_semver = :gs")
            .expression_attribute_values(":gs", AttributeValue::S(greatest_stable_semver))
            .send()
            .await;
    }

    Ok(())
}

fn find_semver(val: &Value) -> String {
    let versions = val["versions"].as_object().unwrap();

    versions
        .keys()
        .filter(|v| {
            let version = Version::parse(v).unwrap();

            version.pre.is_empty()
        })
        .max()
        .unwrap()
        .to_owned()
}
