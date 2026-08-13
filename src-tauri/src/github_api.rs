//! The GitHub Contents API, with errors that stay distinguishable.
//!
//! The distinction that matters: **404 is the only thing that means "empty".**
//! Collapsing an expired token, a 500, or a truncated body into "the remote has
//! nothing" makes the local database look like the whole truth, and the next
//! push publishes that assumption over everyone else's work.

use base64::{engine::general_purpose, Engine as _};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug)]
pub enum GithubError {
    /// The path does not exist. The only "empty" there is.
    NotFound,
    /// The SHA we sent is stale: someone wrote first.
    Conflict,
    Unauthorized,
    RateLimited,
    Status(StatusCode, String),
    Transport(String),
    Decode(String),
}

impl fmt::Display for GithubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "not found"),
            Self::Conflict => write!(f, "the remote changed while we were writing"),
            Self::Unauthorized => write!(
                f,
                "GitHub rejected the token — check that it is valid and has repo access"
            ),
            Self::RateLimited => write!(f, "GitHub rate limit reached; try again shortly"),
            Self::Status(code, body) => {
                let body = body.trim();
                if body.is_empty() {
                    write!(f, "GitHub returned {code}")
                } else {
                    write!(f, "GitHub returned {code}: {body}")
                }
            }
            Self::Transport(e) => write!(f, "network error: {e}"),
            Self::Decode(e) => write!(f, "could not decode the response: {e}"),
        }
    }
}

impl std::error::Error for GithubError {}

impl From<reqwest::Error> for GithubError {
    fn from(error: reqwest::Error) -> Self {
        Self::Transport(error.to_string())
    }
}

#[derive(Serialize)]
struct PutBody {
    message: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sha: Option<String>,
}

#[derive(Deserialize)]
struct FileResponse {
    content: String,
    sha: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DirEntry {
    pub name: String,
    pub sha: String,
    #[serde(rename = "type")]
    pub kind: String,
}

fn contents_url(owner: &str, repo: &str, path: &str) -> String {
    // Each path segment is encoded separately so slashes stay structural.
    let encoded: Vec<String> = path
        .split('/')
        .map(|segment| urlencoding_encode(segment))
        .collect();
    format!(
        "https://api.github.com/repos/{}/{}/contents/{}",
        urlencoding_encode(owner),
        urlencoding_encode(repo),
        encoded.join("/")
    )
}

/// Percent-encode everything that is not unreserved. Small enough not to earn a
/// dependency, and it keeps a repo or device name with a space from silently
/// producing a different URL than intended.
fn urlencoding_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn classify(status: StatusCode, body: String) -> GithubError {
    match status {
        StatusCode::NOT_FOUND => GithubError::NotFound,
        StatusCode::UNAUTHORIZED => GithubError::Unauthorized,
        StatusCode::CONFLICT => GithubError::Conflict,
        // GitHub answers a stale SHA with 422, and a rate limit with 403.
        StatusCode::UNPROCESSABLE_ENTITY => GithubError::Conflict,
        StatusCode::FORBIDDEN if body.contains("rate limit") => GithubError::RateLimited,
        StatusCode::FORBIDDEN => GithubError::Unauthorized,
        other => GithubError::Status(other, body),
    }
}

fn client() -> Client {
    Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default()
}

/// `Ok(None)` means the file genuinely is not there.
pub async fn get_file(
    token: &str,
    owner: &str,
    repo: &str,
    path: &str,
) -> Result<Option<(String, String)>, GithubError> {
    let response = client()
        .get(contents_url(owner, repo, path))
        .header(USER_AGENT, "yClippy")
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(ACCEPT, "application/vnd.github+json")
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return match classify(status, body) {
            GithubError::NotFound => Ok(None),
            other => Err(other),
        };
    }

    let body: FileResponse = response
        .json()
        .await
        .map_err(|e| GithubError::Decode(e.to_string()))?;
    let cleaned: String = body.content.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = general_purpose::STANDARD
        .decode(&cleaned)
        .map_err(|e| GithubError::Decode(e.to_string()))?;
    let text = String::from_utf8(bytes).map_err(|e| GithubError::Decode(e.to_string()))?;
    Ok(Some((text, body.sha)))
}

pub async fn list_dir(
    token: &str,
    owner: &str,
    repo: &str,
    path: &str,
) -> Result<Vec<DirEntry>, GithubError> {
    let response = client()
        .get(contents_url(owner, repo, path))
        .header(USER_AGENT, "yClippy")
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(ACCEPT, "application/vnd.github+json")
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(classify(status, body));
    }

    let entries: Vec<DirEntry> = response
        .json()
        .await
        .map_err(|e| GithubError::Decode(e.to_string()))?;
    Ok(entries.into_iter().filter(|e| e.kind == "file").collect())
}

/// Passing `sha` makes the write conditional: a stale one comes back as
/// [`GithubError::Conflict`] rather than overwriting.
pub async fn put_file(
    token: &str,
    owner: &str,
    repo: &str,
    path: &str,
    content: &str,
    sha: Option<String>,
    message: &str,
) -> Result<(), GithubError> {
    let body = PutBody {
        message: message.to_string(),
        content: general_purpose::STANDARD.encode(content),
        sha,
    };

    let response = client()
        .put(contents_url(owner, repo, path))
        .header(USER_AGENT, "yClippy")
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(ACCEPT, "application/vnd.github+json")
        .header(CONTENT_TYPE, "application/json")
        .json(&body)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(classify(status, text));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_encoded_per_segment_so_slashes_stay_structural() {
        let url = contents_url("owner", "repo", ".notes/yclippy/library.json");
        assert_eq!(
            url,
            "https://api.github.com/repos/owner/repo/contents/.notes/yclippy/library.json"
        );
    }

    #[test]
    fn a_device_name_with_awkward_characters_cannot_reshape_the_url() {
        let url = contents_url("owner", "repo", ".notes/yclippy/devices/a b?c.jsonl");
        assert!(url.ends_with("/devices/a%20b%3Fc.jsonl"), "got {url}");
    }

    #[test]
    fn only_404_reads_as_empty() {
        assert!(matches!(
            classify(StatusCode::NOT_FOUND, String::new()),
            GithubError::NotFound
        ));
        // The ones the old code silently treated as "the remote is empty".
        assert!(matches!(
            classify(StatusCode::UNAUTHORIZED, String::new()),
            GithubError::Unauthorized
        ));
        assert!(matches!(
            classify(StatusCode::INTERNAL_SERVER_ERROR, String::new()),
            GithubError::Status(..)
        ));
        assert!(matches!(
            classify(StatusCode::UNPROCESSABLE_ENTITY, String::new()),
            GithubError::Conflict
        ));
        assert!(matches!(
            classify(StatusCode::FORBIDDEN, "API rate limit exceeded".into()),
            GithubError::RateLimited
        ));
    }

    #[test]
    fn errors_say_what_to_do_about_them() {
        assert!(GithubError::Unauthorized.to_string().contains("token"));
        assert!(GithubError::RateLimited.to_string().contains("rate limit"));
    }
}
