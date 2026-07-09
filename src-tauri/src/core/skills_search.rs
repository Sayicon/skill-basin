use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::Deserialize;

/// skills.sh is an unofficial API: it can be slow, down, or change shape
/// without notice. Bound the wait, buffer the answers, and let callers fall
/// back to GitHub search when it fails.
const REQUEST_TIMEOUT_SECS: u64 = 10;
pub(crate) const CACHE_TTL_SECS: u64 = 300;
pub(crate) const CACHE_CAPACITY: usize = 32;

#[derive(Debug, Deserialize)]
struct SkillsShResponse {
    skills: Vec<SkillsShItem>,
}

#[derive(Debug, Deserialize)]
struct SkillsShItem {
    name: String,
    installs: u64,
    source: String,
    /// Absent on most entries today. Never inferred — an unknown license is
    /// reported as unknown so the UI can warn instead of implying a grant.
    #[serde(default)]
    license: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OnlineSkillResult {
    pub name: String,
    pub installs: u64,
    pub source: String,
    pub source_url: String,
    pub license: Option<String>,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    stored_at: u64,
    results: Vec<OnlineSkillResult>,
}

/// Tiny fixed-capacity TTL cache keyed by (query, limit). Time is passed in
/// rather than read from the clock so the eviction rules stay testable.
#[derive(Debug, Default)]
pub(crate) struct SearchCache {
    entries: HashMap<(String, usize), CacheEntry>,
}

impl SearchCache {
    pub(crate) fn get(
        &mut self,
        query: &str,
        limit: usize,
        now_secs: u64,
    ) -> Option<Vec<OnlineSkillResult>> {
        let key = (query.to_string(), limit);
        let entry = self.entries.get(&key)?;
        if now_secs.saturating_sub(entry.stored_at) >= CACHE_TTL_SECS {
            self.entries.remove(&key);
            return None;
        }
        Some(entry.results.clone())
    }

    pub(crate) fn put(
        &mut self,
        query: &str,
        limit: usize,
        results: Vec<OnlineSkillResult>,
        now_secs: u64,
    ) {
        if self.entries.len() >= CACHE_CAPACITY {
            if let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.stored_at)
                .map(|(key, _)| key.clone())
            {
                self.entries.remove(&oldest);
            }
        }
        self.entries.insert(
            (query.to_string(), limit),
            CacheEntry {
                stored_at: now_secs,
                results,
            },
        );
    }
}

fn cache() -> &'static Mutex<SearchCache> {
    static CACHE: std::sync::OnceLock<Mutex<SearchCache>> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(SearchCache::default()))
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

/// Cached search against skills.sh. A poisoned cache lock must never take the
/// search down, so lock failures simply skip the cache.
pub fn search_skills_online(query: &str, limit: usize) -> Result<Vec<OnlineSkillResult>> {
    let now = now_secs();
    if let Ok(mut guard) = cache().lock() {
        if let Some(hit) = guard.get(query, limit, now) {
            return Ok(hit);
        }
    }

    let results = search_skills_online_inner("https://skills.sh", query, limit)?;

    if let Ok(mut guard) = cache().lock() {
        guard.put(query, limit, results.clone(), now);
    }
    Ok(results)
}

fn search_skills_online_inner(
    base_url: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<OnlineSkillResult>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .context("build skills.sh client")?;
    let base_url = base_url.trim_end_matches('/');
    let url = format!(
        "{}/api/search?q={}&limit={}",
        base_url,
        urlencoding::encode(query),
        limit.clamp(1, 50)
    );

    let response = client
        .get(url)
        .header("User-Agent", "skillbasin")
        .send()
        .context("skills.sh search request failed")?
        .error_for_status()
        .context("skills.sh search returned error")?;

    let result: SkillsShResponse = response.json().context("parse skills.sh response")?;

    Ok(result
        .skills
        .into_iter()
        .map(|item| {
            let source_url = format!("https://github.com/{}", item.source);
            OnlineSkillResult {
                name: item.name,
                installs: item.installs,
                source: item.source,
                source_url,
                license: item
                    .license
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
            }
        })
        .collect())
}

#[cfg(test)]
#[path = "tests/skills_search.rs"]
mod tests;
