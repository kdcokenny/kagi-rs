use kagi_sdk::{
    official_api::models::{SearchRequest, SummarizePostInput, SummarizePostRequest},
    parsing::{parse_html_search_response, parse_summary_stream_response},
    routing::{ApiVersion, EndpointId, ProtocolSurface},
    session_web::models::{HtmlSearchRequest, SummaryLabsTextRequest, SummaryLabsUrlRequest},
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
        .search(SearchRequest::new("rust").expect("query valid"))
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
        .html_search(HtmlSearchRequest::new("rust").expect("query valid"))
        .await
        .expect("html search should parse");

    mock.assert();
    assert_eq!(response.results.len(), 2);
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
        .search(SearchRequest::new("rust").expect("query valid"))
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
        .html_search(HtmlSearchRequest::new("rust").expect("query valid"))
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
        .html_search(HtmlSearchRequest::new("rust").expect("query valid"))
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
        .with_body("<html><body>please sign in</body></html>")
        .create();

    let client = KagiClient::builder()
        .config(test_config(&server))
        .session_token(SessionToken::new("session_token_123").expect("token is valid"))
        .build()
        .expect("client builds");

    let result = client
        .session_web()
        .expect("session web available")
        .summary_labs_url(
            SummaryLabsUrlRequest::new("https://example.com/post").expect("url valid"),
        )
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
        .summary_labs_text(SummaryLabsTextRequest::new("Hello from sdk").expect("text valid"))
        .await
        .expect("summary labs post should parse");

    assert_eq!(response.text, "Hello from SDK");
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
        .search(SearchRequest::new("rust").expect("query valid"))
        .await
        .expect("response should parse");

    assert_eq!(result.data["ok"], true);
}
