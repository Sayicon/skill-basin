use mockito::Matcher;

use super::*;

fn json_response() -> String {
    r#"{
  "skills": [
    {
      "name": "react-expert",
      "installs": 203000,
      "source": "vercel-labs/agent-skills",
      "license": "MIT"
    },
    {
      "name": "vue-master",
      "installs": 57000,
      "source": "vuejs/vue-skills"
    }
  ],
  "count": 2
}"#
    .to_string()
}

fn json_empty() -> String {
    r#"{"skills": [], "count": 0}"#.to_string()
}

#[test]
fn parses_search_results() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/api/search")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("q".into(), "react".into()),
            Matcher::UrlEncoded("limit".into(), "20".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(json_response())
        .create();

    let out = search_skills_online_inner(&server.url(), "react", 20).unwrap();
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].name, "react-expert");
    assert_eq!(out[0].installs, 203000);
    assert_eq!(out[0].source, "vercel-labs/agent-skills");
    assert_eq!(
        out[0].source_url,
        "https://github.com/vercel-labs/agent-skills"
    );
}

#[test]
fn source_url_is_constructed_from_source() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/api/search")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("q".into(), "vue".into()),
            Matcher::UrlEncoded("limit".into(), "5".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(json_response())
        .create();

    let out = search_skills_online_inner(&server.url(), "vue", 5).unwrap();
    assert_eq!(out[1].source_url, "https://github.com/vuejs/vue-skills");
}

#[test]
fn http_error_returns_error() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/api/search")
        .with_status(500)
        .with_body("internal error")
        .create();

    let err = search_skills_online_inner(&server.url(), "test", 10).unwrap_err();
    let msg = format!("{:#}", err);
    assert!(msg.contains("skills.sh search returned error"), "{}", msg);
}

#[test]
fn empty_results() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/api/search")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("q".into(), "nonexistent".into()),
            Matcher::UrlEncoded("limit".into(), "10".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(json_empty())
        .create();

    let out = search_skills_online_inner(&server.url(), "nonexistent", 10).unwrap();
    assert!(out.is_empty());
}

// ── License: an unofficial API may or may not report one ─────────────────

#[test]
fn license_is_carried_through_when_present_and_absent() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/api/search")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(json_response())
        .create();

    let out = search_skills_online_inner(&server.url(), "any", 10).unwrap();
    assert_eq!(out[0].license.as_deref(), Some("MIT"));
    // Missing license must stay None so the UI can warn, never guess a license.
    assert_eq!(out[1].license, None);
}

#[test]
fn blank_license_string_is_treated_as_unknown() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/api/search")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"skills":[{"name":"x","installs":1,"source":"a/b","license":"  "}]}"#)
        .create();

    let out = search_skills_online_inner(&server.url(), "x", 10).unwrap();
    assert_eq!(out[0].license, None);
}

// ── Cache: an unofficial API needs a local buffer ─────────────────────────

#[test]
fn cache_returns_stored_results_without_a_second_request() {
    let mut cache = SearchCache::default();
    let hits = vec![OnlineSkillResult {
        name: "cached".to_string(),
        installs: 1,
        source: "a/b".to_string(),
        source_url: "https://github.com/a/b".to_string(),
        license: None,
    }];

    assert!(cache.get("react", 20, 100).is_none());
    cache.put("react", 20, hits.clone(), 100);

    let cached = cache.get("react", 20, 150).expect("still fresh");
    assert_eq!(cached[0].name, "cached");
    // Same query, different limit is a different request.
    assert!(cache.get("react", 5, 150).is_none());
}

#[test]
fn cache_entry_expires_after_its_ttl() {
    let mut cache = SearchCache::default();
    cache.put("react", 20, Vec::new(), 100);

    assert!(cache.get("react", 20, 100 + CACHE_TTL_SECS - 1).is_some());
    assert!(cache.get("react", 20, 100 + CACHE_TTL_SECS).is_none());
}

#[test]
fn cache_evicts_the_oldest_entry_when_full() {
    let mut cache = SearchCache::default();
    for i in 0..CACHE_CAPACITY {
        cache.put(&format!("q{i}"), 20, Vec::new(), 100 + i as u64);
    }
    assert!(cache.get("q0", 20, 100).is_some());

    cache.put("overflow", 20, Vec::new(), 999);
    assert!(cache.get("q0", 20, 100).is_none(), "oldest evicted");
    assert!(cache.get("overflow", 20, 999).is_some());
}
