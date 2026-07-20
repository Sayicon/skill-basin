use anyhow::{Context, Result};
use serde::Deserialize;

use super::network_proxy::github_http_client;

/// Search must not hang the UI: GitHub's search API is slow under load and a
/// client with no timeout waits forever.
const SEARCH_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Deserialize)]
struct SearchResponse {
    items: Vec<RepoItem>,
}

#[derive(Debug, Deserialize)]
struct RepoItem {
    full_name: String,
    html_url: String,
    description: Option<String>,
    stargazers_count: u64,
    updated_at: String,
    clone_url: String,
    #[serde(default)]
    license: Option<RepoLicense>,
}

#[derive(Debug, Deserialize)]
struct RepoLicense {
    #[serde(default)]
    spdx_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RepoSummary {
    pub full_name: String,
    pub html_url: String,
    pub description: Option<String>,
    pub stars: u64,
    pub updated_at: String,
    pub clone_url: String,
    /// SPDX id, or `None` when GitHub cannot identify one. Installing an
    /// unlicensed skill is a decision for the user to make knowingly.
    pub license: Option<String>,
}

/// GitHub returns this when it finds a license file it cannot classify.
/// It is not a license grant, so it must not be shown as one.
const SPDX_UNKNOWN: &str = "NOASSERTION";

fn normalize_spdx(license: Option<RepoLicense>) -> Option<String> {
    license
        .and_then(|entry| entry.spdx_id)
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty() && !id.eq_ignore_ascii_case(SPDX_UNKNOWN))
}

pub fn search_github_repos(
    query: &str,
    limit: usize,
    token: Option<&str>,
    proxy_url: &str,
) -> Result<Vec<RepoSummary>> {
    search_github_repos_inner("https://api.github.com", query, limit, token, proxy_url)
}

pub(super) fn search_github_repos_inner(
    base_url: &str,
    query: &str,
    limit: usize,
    token: Option<&str>,
    proxy_url: &str,
) -> Result<Vec<RepoSummary>> {
    // Search is the endpoint most likely to hit GitHub's rate limit (10/min
    // unauthenticated), and a request with no timeout can hang the UI forever.
    let client = github_http_client(proxy_url, Some(SEARCH_TIMEOUT_SECS))?;
    let base_url = base_url.trim_end_matches('/');
    let url = format!(
        "{}/search/repositories?q={}&per_page={}",
        base_url,
        urlencoding::encode(query),
        limit.clamp(1, 50)
    );

    let mut req = client.get(url).header("User-Agent", "skills-hub");
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }
    // Route through the shared checker so a 403 rate-limit surfaces as
    // RATE_LIMITED|<mins> here too, not just on the download path.
    let response = crate::core::github_download::check_github_response(
        req.send().context("GitHub search request failed")?,
        "GitHub search",
    )?;

    let result: SearchResponse = response.json().context("parse GitHub response")?;

    Ok(result
        .items
        .into_iter()
        .map(|item| RepoSummary {
            full_name: item.full_name,
            html_url: item.html_url,
            description: item.description,
            stars: item.stargazers_count,
            updated_at: item.updated_at,
            clone_url: item.clone_url,
            license: normalize_spdx(item.license),
        })
        .collect())
}

#[cfg(test)]
#[path = "tests/github_search.rs"]
mod tests;
