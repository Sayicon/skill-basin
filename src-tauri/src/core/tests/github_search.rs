use mockito::Matcher;

use super::search_github_repos_inner;

fn json_one_repo() -> String {
    r#"{
  "items": [
    {
      "full_name": "o/r",
      "html_url": "https://example.com/o/r",
      "description": "d",
      "stargazers_count": 123,
      "updated_at": "2020-01-01T00:00:00Z",
      "clone_url": "https://example.com/o/r.git"
    }
  ]
}"#
    .to_string()
}

#[test]
fn license_is_read_from_spdx_id_and_stays_unknown_when_absent() {
    let body = r#"{
  "items": [
    {
      "full_name": "o/licensed",
      "html_url": "https://example.com/a",
      "description": null,
      "stargazers_count": 1,
      "updated_at": "2020-01-01T00:00:00Z",
      "clone_url": "https://example.com/a.git",
      "license": { "spdx_id": "MIT", "name": "MIT License" }
    },
    {
      "full_name": "o/unlicensed",
      "html_url": "https://example.com/b",
      "description": null,
      "stargazers_count": 1,
      "updated_at": "2020-01-01T00:00:00Z",
      "clone_url": "https://example.com/b.git",
      "license": null
    },
    {
      "full_name": "o/noassertion",
      "html_url": "https://example.com/c",
      "description": null,
      "stargazers_count": 1,
      "updated_at": "2020-01-01T00:00:00Z",
      "clone_url": "https://example.com/c.git",
      "license": { "spdx_id": "NOASSERTION", "name": "Other" }
    }
  ]
}"#;
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/search/repositories")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create();

    let out = search_github_repos_inner(&server.url(), "q", 10, None, "").unwrap();
    assert_eq!(out[0].license.as_deref(), Some("MIT"));
    assert_eq!(out[1].license, None);
    // GitHub reports NOASSERTION for repos whose license it cannot identify;
    // that is not a license, and claiming one would mislead the user.
    assert_eq!(out[2].license, None);
}

#[test]
fn limit_is_clamped() {
    let mut server = mockito::Server::new();

    let _m1 = server
        .mock("GET", "/search/repositories")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("q".into(), "hello".into()),
            Matcher::UrlEncoded("per_page".into(), "1".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(json_one_repo())
        .create();

    let out = search_github_repos_inner(&server.url(), "hello", 0, None, "").unwrap();
    assert_eq!(out.len(), 1);

    let _m2 = server
        .mock("GET", "/search/repositories")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("q".into(), "hello".into()),
            Matcher::UrlEncoded("per_page".into(), "50".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(json_one_repo())
        .create();

    let _ = search_github_repos_inner(&server.url(), "hello", 999, None, "").unwrap();
}

#[test]
fn maps_fields() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/search/repositories")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("q".into(), "x".into()),
            Matcher::UrlEncoded("per_page".into(), "2".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(json_one_repo())
        .create();

    let out = search_github_repos_inner(&server.url(), "x", 2, None, "").unwrap();
    assert_eq!(out[0].full_name, "o/r");
    assert_eq!(out[0].stars, 123);
}

#[test]
fn http_error_has_context() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/search/repositories")
        .with_status(500)
        .with_body("oops")
        .create();

    let err = search_github_repos_inner(&server.url(), "x", 2, None, "").unwrap_err();
    let msg = format!("{:#}", err);
    assert!(msg.contains("GitHub search returned error"), "{msg}");
}
