use aws_sdk_dynamodb::model::AttributeValue;
use aws_types::region::Region;
use chrono::{Duration, Utc};
use esm_checker::StatsEntry;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let discord_webhook = std::env::var("DISCORD_WEBHOOK").unwrap();

    let stats_table_name = std::env::var("DYNAMO_STATS_TABLE_NAME")
        .expect("There should be a stats table name defined as an environment variable");

    let aws_region = std::env::var("AWS_REGION").ok().map(Region::new).unwrap();
    let config = aws_config::from_env().region(aws_region).load().await;
    let dynamo_client = aws_sdk_dynamodb::Client::new(&config);

    let today = Utc::now();
    let last_week = Utc::now().checked_sub_signed(Duration::weeks(1)).unwrap();

    let today_pk_str = today.format("%Y-%m").to_string();
    let last_week_pk_str = last_week.format("%Y-%m").to_string();

    let today_sk_str = today.format("%Y-%m-%d").to_string();
    let last_week_sk_str = last_week.format("%Y-%m-%d").to_string();

    let today_item = fetch_stats_entry(
        &dynamo_client,
        &stats_table_name,
        today_pk_str,
        today_sk_str,
    )
    .await?;

    let last_week_item = fetch_stats_entry(
        &dynamo_client,
        &stats_table_name,
        last_week_pk_str,
        last_week_sk_str,
    )
    .await?;

    let diff = today_item - last_week_item;

    let diff_str = format!(
        "[ESM Checker]\nStats for {}\ntype_module: {}\nexports_require: {}\nexports_no_require: {}",
        diff.timestamp, diff.type_module, diff.exports_require, diff.exports_no_require
    );

    let body = serde_json::json!({ "content": format!("{}", diff_str) }).to_string();

    let reqwest_client = reqwest::Client::builder()
        .user_agent("esm-checker-discord-webhook/0.3.1")
        .build()
        .unwrap();

    reqwest_client
        .post(discord_webhook)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await?;

    Ok(())
}

async fn fetch_stats_entry(
    client: &aws_sdk_dynamodb::Client,
    table_name: &String,
    pk: String,
    sk: String,
) -> Result<StatsEntry, Box<dyn std::error::Error>> {
    let query_output = client
        .query()
        .table_name(table_name)
        .key_condition_expression("year_month = :ym and #timestamp = :t")
        .expression_attribute_names("#timestamp", "timestamp")
        .expression_attribute_values(":ym", AttributeValue::S(pk))
        .expression_attribute_values(":t", AttributeValue::S(sk))
        .send()
        .await?;

    let res = query_output
        .items
        .unwrap()
        .first()
        .unwrap()
        .to_owned()
        .into();

    Ok(res)
}
