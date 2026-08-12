use base64::{engine::general_purpose, Engine as _};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Serialize)]
struct UpdateFileBody {
    message: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sha: Option<String>,
}

#[derive(Deserialize)]
struct GetFileResponse {
    content: String,
    sha: String,
}

pub async fn get_file_content(
    token: &str,
    owner: &str,
    repo: &str,
    path: &str,
) -> Result<Option<(String, String)>, Box<dyn Error + Send + Sync>> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/contents/{}",
        owner, repo, path
    );
    let client = Client::new();

    let resp = client
        .get(&url)
        .header(USER_AGENT, "yClippy")
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .header(ACCEPT, "application/vnd.github.v3+json")
        .send()
        .await?;

    if resp.status() == 404 {
        return Ok(None);
    }

    if !resp.status().is_success() {
        return Err(format!("GitHub API error: {}", resp.status()).into());
    }

    let body: GetFileResponse = resp.json().await?;

    // Decode content (it comes with newlines usually)
    let clean_content = body.content.replace("\n", "");
    let decoded_bytes = general_purpose::STANDARD.decode(&clean_content)?;
    let decoded_str = String::from_utf8(decoded_bytes)?;

    Ok(Some((decoded_str, body.sha)))
}

pub async fn update_file(
    token: &str,
    owner: &str,
    repo: &str,
    path: &str,
    content: &str,
    sha: Option<String>,
    message: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/contents/{}",
        owner, repo, path
    );
    let client = Client::new();

    let encoded_content = general_purpose::STANDARD.encode(content);

    let body = UpdateFileBody {
        message: message.to_string(),
        content: encoded_content,
        sha,
    };

    let resp = client
        .put(&url)
        .header(USER_AGENT, "yClippy")
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .header(ACCEPT, "application/vnd.github.v3+json")
        .header(CONTENT_TYPE, "application/json")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await?;
        return Err(format!("GitHub API update error: {}", text).into());
    }

    Ok(())
}
