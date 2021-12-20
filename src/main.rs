use hubcaps::{content::Contents, Credentials, Github};
use serde_json::Value;

async fn get_repo_package_json(
    client: &Github,
    owner: &str,
    repo: &str,
    ref_: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let repo = client.repo(owner, repo);

    let content = repo
        .content()
        .get("package.json", ref_)
        .await
        .expect("Repo did not contain package.json");

    let contents = match content {
        Contents::File(file) => {
            let c = file.content;
            Some(String::from_utf8(c.to_vec()).unwrap())
        }
        _ => None,
    }
    .unwrap();

    let package_json: Value = serde_json::from_str(&contents).unwrap();

    Ok(package_json)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Github::new(
        String::from("github-esm-checker v0.1.0"),
        Credentials::Token(String::from(std::env::var("GITHUB_TOKEN").unwrap())),
    )
    .expect("Should connect to client");

    let package_json = get_repo_package_json(&client, "node-fetch", "node-fetch", "main").await?;

    let package_type = package_json.get("type").unwrap();

    match package_type {
        Value::String(str) => {
            println!("Type: {}", str);
        }
        _ => {}
    }

    dbg!(package_json);

    let core_ratelimit_remaining = client.rate_limit().get().await?.resources.core.remaining;

    dbg!(core_ratelimit_remaining);

    Ok(())
}
