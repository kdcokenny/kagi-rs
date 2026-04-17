use kagi_sdk::{
    official_api::models::{
        SearchRequest as OfficialSearchRequest, SummarizePostInput, SummarizePostRequest,
    },
    parsing::{parse_html_search_response, parse_summary_stream_response},
    routing::{ApiVersion, EndpointId, ProtocolSurface},
    session_web::models::{SearchRequest as SessionSearchRequest, SummarizeRequest, SummaryType},
    BotToken, ClientConfig, KagiClient, KagiError, SessionToken,
};
use mockito::{Matcher, Server};
use url::Url;

fn test_config(server: &Server) -> ClientConfig {
    ClientConfig::default()
        .with_base_url(Url::parse(&server.url()).expect("mockito always emits valid URL"))
}

#[test]
fn routing_matrix_mixes_v0_and_v1_routes() {
    let search = EndpointId::OfficialSearch.spec();
    let feed = EndpointId::OfficialSmallwebFeed.spec();

    assert_eq!(search.version, ApiVersion::V0);
    assert_eq!(search.route, "/api/v0/search");
    assert_eq!(feed.version, ApiVersion::V1);
    assert_eq!(feed.route, "/api/v1/smallweb/feed");
}

#[test]
fn summarize_input_modes_enforce_boundaries() {
    assert!(SummarizePostInput::from_text("   ").is_err());
    assert!(SummarizePostInput::from_url("ftp://example.com").is_err());

    let valid = SummarizePostRequest::from_text("Summarize me");
    assert!(valid.is_ok());
}

#[test]
fn unsupported_auth_surface_combinations_fail_loudly() {
    let bot_client = KagiClient::with_bot_token(BotToken::new("bot_token").expect("valid token"))
        .expect("client should build");
    let session_client =
        KagiClient::with_session_token(SessionToken::new("session_token").expect("valid token"))
            .expect("client should build");

    let bot_to_session = bot_client.session_web();
    let session_to_official = session_client.official_api();

    assert!(matches!(
        bot_to_session,
        Err(KagiError::UnsupportedAuthSurface {
            surface: ProtocolSurface::SessionWeb,
            ..
        })
    ));
    assert!(matches!(
        session_to_official,
        Err(KagiError::UnsupportedAuthSurface {
            surface: ProtocolSurface::OfficialApi,
            ..
        })
    ));
}

#[test]
fn html_parser_is_stable_on_fixture() {
    let fixture = include_str!("fixtures/html_search_fixture.html");
    let parsed = parse_html_search_response(EndpointId::SessionHtmlSearch, fixture)
        .expect("fixture should parse");

    assert_eq!(parsed.results.len(), 2);
    assert_eq!(parsed.results[0].title, "Example Result One");
    assert_eq!(parsed.results[0].url, "https://example.com/one");
}

#[test]
fn summary_stream_parser_is_stable_on_fixture() {
    let fixture = include_str!("fixtures/summary_stream_fixture.txt");
    let parsed = parse_summary_stream_response(EndpointId::SessionSummaryLabsGet, fixture)
        .expect("fixture should parse");

    assert_eq!(parsed.chunks.len(), 3);
    assert_eq!(parsed.text, "Hello world!");
}

#[test]
fn summary_stream_parser_supports_kagi_prefixed_frames() {
    let framed_stream =
        "hi:{}\0new_message.json:{\"text\":\"Hello\"}\0update: \0update:world\0final:{\"status\":\"done\"}\0";

    let parsed = parse_summary_stream_response(EndpointId::SessionSummaryLabsGet, framed_stream)
        .expect("Kagi framed stream should parse");

    assert_eq!(parsed.chunks, vec!["Hello", " ", "world"]);
    assert_eq!(parsed.text, "Hello world");
}

#[test]
fn summary_stream_parser_rejects_unsupported_frame_prefixes() {
    let framed_stream = "unknown:payload\0";
    let parsed = parse_summary_stream_response(EndpointId::SessionSummaryLabsGet, framed_stream);

    assert!(matches!(parsed, Err(KagiError::ResponseParse { .. })));
}

#[test]
fn html_parser_rejects_non_result_markup() {
    let markup = r#"
        <html>
          <body>
            <nav><a href="https://kagi.com/login">Sign in</a></nav>
            <ul><li><a href="https://example.com">Random link</a></li></ul>
          </body>
        </html>
    "#;

    let parsed = parse_html_search_response(EndpointId::SessionHtmlSearch, markup);
    assert!(matches!(parsed, Err(KagiError::ResponseParse { .. })));
}

#[test]
fn html_parser_returns_empty_results_for_no_result_pages() {
    let markup = r#"
        <html>
          <body>
            <form action="/html/search"><input name="q" /></form>
            <p>No results found for your query.</p>
          </body>
        </html>
    "#;

    let parsed = parse_html_search_response(EndpointId::SessionHtmlSearch, markup)
        .expect("empty search pages should parse");
    assert!(parsed.results.is_empty());
}

#[test]
fn summary_stream_parser_rejects_non_sse_bodies() {
    let invalid_body = "this is not an sse payload";
    let parsed = parse_summary_stream_response(EndpointId::SessionSummaryLabsGet, invalid_body);

    assert!(matches!(parsed, Err(KagiError::ResponseParse { .. })));
}

#[test]
fn summary_stream_parser_ignores_metadata_and_handles_error_events() {
    let metadata_stream = "id: 1\nevent: message\ndata: {\"text\":\"Hello\"}\n\nretry: 1000\ndata: {\"text\":\" world\"}\n\n";
    let parsed = parse_summary_stream_response(EndpointId::SessionSummaryLabsGet, metadata_stream)
        .expect("metadata stream should parse");

    assert_eq!(parsed.text, "Hello world");

    let error_stream =
        "event: error\ndata: {\"code\":\"UPSTREAM\",\"message\":\"generation failed\"}\n\n";
    let error = parse_summary_stream_response(EndpointId::SessionSummaryLabsGet, error_stream);
    assert!(matches!(
        error,
        Err(KagiError::ApiFailure {
            code: Some(ref code),
            ..
        }) if code == "UPSTREAM"
    ));
}

#[test]
fn summary_stream_parser_preserves_whitespace_only_chunks() {
    let framed_stream = "update:Hello\0update: \0update:world\0";
    let parsed = parse_summary_stream_response(EndpointId::SessionSummaryLabsGet, framed_stream)
        .expect("stream should parse");

    assert_eq!(parsed.chunks, vec!["Hello", " ", "world"]);
    assert_eq!(parsed.text, "Hello world");
}

#[test]
fn builder_rejects_conflicting_credential_setup() {
    let result = KagiClient::builder()
        .bot_token(BotToken::new("bot_token").expect("token is valid"))
        .session_token(SessionToken::new("session_token").expect("token is valid"))
        .build();

    assert!(matches!(
        result,
        Err(KagiError::ConflictingCredentialConfiguration { .. })
    ));
}

#[tokio::test]
async fn official_surface_injects_bot_token_header() {
    let mut server = Server::new();
    let token = "bot_token_123";

    let mock = server
        .mock("GET", "/api/v0/search")
        .match_query(Matcher::UrlEncoded("q".into(), "rust".into()))
        .match_header("authorization", Matcher::Exact(format!("Bot {token}")))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"data":{"ok":true}}"#)
        .create();

    let client = KagiClient::builder()
        .config(test_config(&server))
        .bot_token(BotToken::new(token).expect("token is valid"))
        .build()
        .expect("client builds");

    let response = client
        .official_api()
        .expect("official api available")
        .search(OfficialSearchRequest::new("rust").expect("query valid"))
        .await
        .expect("search should succeed");

    mock.assert();
    assert_eq!(response.data["ok"], true);
}

#[tokio::test]
async fn session_surface_injects_session_cookie() {
    let mut server = Server::new();
    let token = "session_token_123";

    let fixture = include_str!("fixtures/html_search_fixture.html");
    let mock = server
        .mock("GET", "/html/search")
        .match_query(Matcher::UrlEncoded("q".into(), "rust".into()))
        .match_header("cookie", Matcher::Exact(format!("kagi_session={token}")))
        .with_status(200)
        .with_body(fixture)
        .create();

    let client = KagiClient::builder()
        .config(test_config(&server))
        .session_token(SessionToken::new(token).expect("token is valid"))
        .build()
        .expect("client builds");

    let response = client
        .session_web()
        .expect("session web available")
        .search(SessionSearchRequest::new("rust").expect("query valid"))
        .await
        .expect("html search should parse");

    mock.assert();
    assert_eq!(response.results.len(), 2);
}

#[tokio::test]
async fn search_results_with_auth_like_substrings_are_not_invalid_session() {
    let mut server = Server::new();
    let token = "session_token_123";

    let body = r#"
        <html>
          <body>
            <div class="search-result __sri">
              <a class="__sri_title_link" href="https://example.com/post">Result page</a>
              <div class="__sri-desc">This article mentions /auth/login and says please sign in.</div>
            </div>
          </body>
        </html>
    "#;

    let _mock = server
        .mock("GET", "/html/search")
        .match_query(Matcher::UrlEncoded("q".into(), "rust".into()))
        .match_header("cookie", Matcher::Exact(format!("kagi_session={token}")))
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(body)
        .create();

    let client = KagiClient::builder()
        .config(test_config(&server))
        .session_token(SessionToken::new(token).expect("token is valid"))
        .build()
        .expect("client builds");

    let result = client
        .session_web()
        .expect("session web available")
        .search(SessionSearchRequest::new("rust").expect("query valid"))
        .await
        .expect("search should parse");

    assert_eq!(result.results.len(), 1);
}

#[tokio::test]
async fn unauthorized_bot_token_maps_to_typed_error() {
    let mut server = Server::new();
    let _mock = server
        .mock("GET", "/api/v0/search")
        .match_query(Matcher::UrlEncoded("q".into(), "rust".into()))
        .with_status(401)
        .with_body("bad bot token")
        .create();

    let client = KagiClient::builder()
        .config(test_config(&server))
        .bot_token(BotToken::new("bot_token_123").expect("token is valid"))
        .build()
        .expect("client builds");

    let result = client
        .official_api()
        .expect("official api available")
        .search(OfficialSearchRequest::new("rust").expect("query valid"))
        .await;

    assert!(matches!(
        result,
        Err(KagiError::UnauthorizedBotToken { .. })
    ));
}

#[tokio::test]
async fn invalid_session_maps_to_typed_error() {
    let mut server = Server::new();
    let _mock = server
        .mock("GET", "/html/search")
        .match_query(Matcher::UrlEncoded("q".into(), "rust".into()))
        .with_status(403)
        .with_body("expired session")
        .create();

    let client = KagiClient::builder()
        .config(test_config(&server))
        .session_token(SessionToken::new("session_token_123").expect("token is valid"))
        .build()
        .expect("client builds");

    let result = client
        .session_web()
        .expect("session web available")
        .search(SessionSearchRequest::new("rust").expect("query valid"))
        .await;

    assert!(matches!(
        result,
        Err(KagiError::InvalidSession { status: 403, .. })
    ));
}

#[tokio::test]
async fn session_redirect_response_maps_to_invalid_session() {
    let mut server = Server::new();
    let _mock = server
        .mock("GET", "/html/search")
        .match_query(Matcher::UrlEncoded("q".into(), "rust".into()))
        .with_status(302)
        .with_header("location", "/auth/login")
        .create();

    let client = KagiClient::builder()
        .config(test_config(&server))
        .session_token(SessionToken::new("session_token_123").expect("token is valid"))
        .build()
        .expect("client builds");

    let result = client
        .session_web()
        .expect("session web available")
        .search(SessionSearchRequest::new("rust").expect("query valid"))
        .await;

    assert!(matches!(
        result,
        Err(KagiError::InvalidSession { status: 302, .. })
    ));
}

#[tokio::test]
async fn summary_labs_rejects_html_interstitial_payload() {
    let mut server = Server::new();
    let _mock = server
        .mock("GET", "/mother/summary_labs")
        .match_query(Matcher::UrlEncoded(
            "url".into(),
            "https://example.com/post".into(),
        ))
        .with_status(200)
        .with_header("content-type", "text/html; charset=utf-8")
        .with_body(
            r#"
            <html>
              <head><title>Sign in to Kagi</title></head>
              <body>
                <form action="/auth/login" method="post">
                  <input type="email" name="email" />
                  <input type="password" name="password" />
                </form>
              </body>
            </html>
            "#,
        )
        .create();

    let client = KagiClient::builder()
        .config(test_config(&server))
        .session_token(SessionToken::new("session_token_123").expect("token is valid"))
        .build()
        .expect("client builds");

    let result = client
        .session_web()
        .expect("session web available")
        .summarize(SummarizeRequest::from_url("https://example.com/post").expect("url valid"))
        .await;

    assert!(matches!(result, Err(KagiError::InvalidSession { .. })));
}

#[tokio::test]
async fn session_summary_labs_post_text_round_trip_works() {
    let mut server = Server::new();
    let token = "session_token_123";
    let _mock = server
        .mock("POST", "/mother/summary_labs/")
        .match_header("cookie", Matcher::Exact(format!("kagi_session={token}")))
        .match_body(Matcher::UrlEncoded(
            "text".to_string(),
            "Hello from sdk".to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body("data: {\"text\":\"Hello \"}\ndata: {\"text\":\"from SDK\"}\n")
        .create();

    let client = KagiClient::builder()
        .config(test_config(&server))
        .session_token(SessionToken::new(token).expect("token is valid"))
        .build()
        .expect("client builds");

    let response = client
        .session_web()
        .expect("session web available")
        .summarize_stream(SummarizeRequest::from_text("Hello from sdk").expect("text valid"))
        .await
        .expect("summary labs post should parse");

    assert_eq!(response.text, "Hello from SDK");
}

#[tokio::test]
async fn session_summarize_default_json_returns_structured_response() {
    let mut server = Server::new();
    let token = "session_token_123";
    let _mock = server
        .mock("GET", "/mother/summary_labs")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("url".into(), "https://example.com/post".into()),
            Matcher::UrlEncoded("summary_type".into(), "takeaway".into()),
            Matcher::UrlEncoded("target_language".into(), "es".into()),
        ]))
        .match_header("cookie", Matcher::Exact(format!("kagi_session={token}")))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r###"{"markdown":"## Summary","text":"Short summary","status":"ok","metadata":{"tokens":42}}"###,
        )
        .create();

    let request = SummarizeRequest::from_url("https://example.com/post")
        .expect("url valid")
        .with_summary_type(SummaryType::Takeaway)
        .with_target_language("es")
        .expect("language should parse");

    let client = KagiClient::builder()
        .config(test_config(&server))
        .session_token(SessionToken::new(token).expect("token is valid"))
        .build()
        .expect("client builds");

    let response = client
        .session_web()
        .expect("session web available")
        .summarize(request)
        .await
        .expect("summarize should parse");

    assert_eq!(response.markdown, "## Summary");
    assert_eq!(response.text.as_deref(), Some("Short summary"));
    assert_eq!(response.status.as_deref(), Some("ok"));
    assert_eq!(response.metadata["tokens"], 42);
}

#[tokio::test]
async fn session_summarize_defaults_metadata_to_empty_object() {
    let mut server = Server::new();
    let _mock = server
        .mock("GET", "/mother/summary_labs")
        .match_query(Matcher::UrlEncoded(
            "url".into(),
            "https://example.com/post".into(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r###"{"markdown":"## Summary"}"###)
        .create();

    let client = KagiClient::builder()
        .config(test_config(&server))
        .session_token(SessionToken::new("session_token_123").expect("token is valid"))
        .build()
        .expect("client builds");

    let response = client
        .session_web()
        .expect("session web available")
        .summarize(SummarizeRequest::from_url("https://example.com/post").expect("url valid"))
        .await
        .expect("summarize should parse");

    assert!(response.metadata.is_empty());
}

#[tokio::test]
async fn session_summarize_malformed_json_maps_to_response_parse() {
    let mut server = Server::new();
    let _mock = server
        .mock("GET", "/mother/summary_labs")
        .match_query(Matcher::UrlEncoded(
            "url".into(),
            "https://example.com/post".into(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("not-json")
        .create();

    let client = KagiClient::builder()
        .config(test_config(&server))
        .session_token(SessionToken::new("session_token_123").expect("token is valid"))
        .build()
        .expect("client builds");

    let result = client
        .session_web()
        .expect("session web available")
        .summarize(SummarizeRequest::from_url("https://example.com/post").expect("url valid"))
        .await;

    assert!(matches!(result, Err(KagiError::ResponseParse { .. })));
}

#[tokio::test]
#[allow(deprecated)]
async fn deprecated_wrappers_delegate_to_new_session_methods() {
    let mut server = Server::new();
    let token = "session_token_123";
    let _search_mock = server
        .mock("GET", "/html/search")
        .match_query(Matcher::UrlEncoded("q".into(), "rust".into()))
        .match_header("cookie", Matcher::Exact(format!("kagi_session={token}")))
        .with_status(200)
        .with_body(include_str!("fixtures/html_search_fixture.html"))
        .create();

    let _stream_get_mock = server
        .mock("GET", "/mother/summary_labs")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("url".into(), "https://example.com/post".into()),
            Matcher::UrlEncoded("stream".into(), "1".into()),
        ]))
        .with_status(200)
        .with_body("data: {\"text\":\"hello\"}\n")
        .create();

    let _stream_post_mock = server
        .mock("POST", "/mother/summary_labs/")
        .match_body(Matcher::AllOf(vec![
            Matcher::UrlEncoded("text".into(), "hello".into()),
            Matcher::UrlEncoded("stream".into(), "1".into()),
        ]))
        .with_status(200)
        .with_body("data: {\"text\":\"world\"}\n")
        .create();

    let client = KagiClient::builder()
        .config(test_config(&server))
        .session_token(SessionToken::new(token).expect("token is valid"))
        .build()
        .expect("client builds");

    let session_web = client.session_web().expect("session web available");
    let _ = session_web
        .html_search(SessionSearchRequest::new("rust").expect("query valid"))
        .await
        .expect("deprecated html_search should work");

    let stream_from_url = session_web
        .summary_labs_url(
            kagi_sdk::session_web::models::SummaryLabsUrlRequest::new("https://example.com/post")
                .expect("url valid"),
        )
        .await
        .expect("deprecated summary_labs_url should work");
    assert_eq!(stream_from_url.text, "hello");

    let stream_from_text = session_web
        .summary_labs_text(
            kagi_sdk::session_web::models::SummaryLabsTextRequest::new("hello")
                .expect("text valid"),
        )
        .await
        .expect("deprecated summary_labs_text should work");
    assert_eq!(stream_from_text.text, "world");
}

#[tokio::test]
async fn official_envelope_allows_error_false_marker() {
    let mut server = Server::new();
    let _mock = server
        .mock("GET", "/api/v0/search")
        .match_query(Matcher::UrlEncoded("q".into(), "rust".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error":false,"data":{"ok":true}}"#)
        .create();

    let client = KagiClient::builder()
        .config(test_config(&server))
        .bot_token(BotToken::new("bot_token_123").expect("token is valid"))
        .build()
        .expect("client builds");

    let result = client
        .official_api()
        .expect("official api available")
        .search(OfficialSearchRequest::new("rust").expect("query valid"))
        .await
        .expect("response should parse");

    assert_eq!(result.data["ok"], true);
}
