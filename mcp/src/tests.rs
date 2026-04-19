use std::time::Duration;

use kagi_sdk::ClientConfig;
use mockito::{Matcher, Server};
use rmcp::{
    model::{CallToolRequestParams, ErrorCode},
    ServiceError, ServiceExt,
};
use serde_json::{json, Value};
use url::Url;

use crate::{
    backend::{BackendRuntime, EnvConfig, ENV_API_KEY, ENV_BACKEND_MODE, ENV_SESSION_TOKEN},
    KagiMcpServer,
};

fn build_config(server: &Server) -> ClientConfig {
    ClientConfig::default()
        .with_base_url(Url::parse(&server.url()).expect("mock server URL is always valid"))
}

fn build_official_backend(server: &Server) -> BackendRuntime {
    BackendRuntime::from_env_config(
        EnvConfig {
            backend_mode: Some("official".to_string()),
            api_key: Some("official_token".to_string()),
            session_token: Some("session_token".to_string()),
        },
        build_config(server),
    )
    .expect("official backend should build")
}

fn build_session_backend(server: &Server) -> BackendRuntime {
    BackendRuntime::from_env_config(
        EnvConfig {
            backend_mode: Some("session".to_string()),
            api_key: Some("official_token".to_string()),
            session_token: Some("session_token".to_string()),
        },
        build_config(server),
    )
    .expect("session backend should build")
}

async fn start_server(
    backend: BackendRuntime,
) -> (
    rmcp::service::RunningService<rmcp::RoleClient, ()>,
    tokio::task::JoinHandle<()>,
) {
    let (server_transport, client_transport) = tokio::io::duplex(64 * 1024);
    let server = KagiMcpServer::from_backend(backend).expect("server should construct");

    let handle = tokio::spawn(async move {
        let running = server
            .serve(server_transport)
            .await
            .expect("server should start");
        let _ = running.waiting().await.expect("server should stop cleanly");
    });

    let client = ().serve(client_transport).await.expect("client should connect");
    (client, handle)
}

fn json_object(value: Value) -> serde_json::Map<String, Value> {
    value
        .as_object()
        .cloned()
        .expect("arguments should be a JSON object")
}

fn assert_invalid_params(error: ServiceError) {
    match error {
        ServiceError::McpError(data) => {
            assert_eq!(data.code, ErrorCode::INVALID_PARAMS);
        }
        unexpected => panic!("expected invalid params mcp error, got {unexpected:?}"),
    }
}

fn schema_property<'a>(schema: &'a Value, field: &str, context: &str) -> &'a Value {
    schema
        .get("properties")
        .and_then(Value::as_object)
        .and_then(|properties| properties.get(field))
        .unwrap_or_else(|| panic!("{context} should expose `{field}` property"))
}

fn schema_property_names(schema: &Value, context: &str) -> Vec<String> {
    let mut names = schema
        .get("properties")
        .and_then(Value::as_object)
        .unwrap_or_else(|| panic!("{context} should expose properties"))
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn schema_required_fields(schema: &Value, context: &str) -> Vec<String> {
    let mut required = schema
        .get("required")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("{context} should expose required fields"))
        .iter()
        .map(|field| {
            field
                .as_str()
                .unwrap_or_else(|| panic!("{context} required field should be string"))
                .to_string()
        })
        .collect::<Vec<_>>();
    required.sort();
    required
}

fn assert_additional_properties_false(schema: &Value, context: &str) {
    assert_eq!(
        schema.get("additionalProperties"),
        Some(&Value::Bool(false)),
        "{context} must disallow unknown fields"
    );
}

fn resolve_local_schema_ref<'a>(
    root: &'a Value,
    mut schema: &'a Value,
    context: &str,
) -> &'a Value {
    loop {
        let Some(reference) = schema.get("$ref").and_then(Value::as_str) else {
            return schema;
        };

        let pointer = reference
            .strip_prefix('#')
            .unwrap_or_else(|| panic!("{context} must use a local JSON pointer reference"));
        schema = root
            .pointer(pointer)
            .unwrap_or_else(|| panic!("{context} points to missing schema ref `{reference}`"));
    }
}

fn search_result_html(count: usize) -> String {
    let mut html = String::from("<html><body>");
    for index in 0..count {
        html.push_str(&format!(
            r#"<div class="search-result"><a class="__sri_title_link" href="https://example.com/{index}">Result {index}</a><div class="__sri-desc">Snippet {index}</div></div>"#
        ));
    }
    html.push_str("</body></html>");
    html
}

#[tokio::test]
async fn tool_listing_has_exactly_two_tools_with_read_only_idempotent_metadata() {
    let server = Server::new();
    let backend = build_official_backend(&server);
    let (client, handle) = start_server(backend).await;

    let tools = client
        .peer()
        .list_all_tools()
        .await
        .expect("tools list should succeed");

    let mut names = tools
        .iter()
        .map(|tool| tool.name.to_string())
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(names, vec!["kagi_search", "kagi_summarize"]);

    for tool in tools {
        let annotations = tool.annotations.expect("tool annotations are required");
        assert_eq!(annotations.read_only_hint, Some(true));
        assert_eq!(annotations.idempotent_hint, Some(true));
    }

    let prompts = client
        .peer()
        .list_all_prompts()
        .await
        .expect("prompt list should succeed");
    assert!(prompts.is_empty());

    let resources = client
        .peer()
        .list_all_resources()
        .await
        .expect("resource list should succeed");
    assert!(resources.is_empty());

    client
        .cancel()
        .await
        .expect("client shutdown should succeed");
    handle.await.expect("server join should succeed");
}

#[tokio::test]
async fn tool_schemas_publish_strict_v1_contract() {
    let server = Server::new();
    let backend = build_official_backend(&server);
    let (client, handle) = start_server(backend).await;

    let tools = client
        .peer()
        .list_all_tools()
        .await
        .expect("tools list should succeed");

    let search_tool = tools
        .iter()
        .find(|tool| tool.name == "kagi_search")
        .expect("kagi_search tool should exist");
    let search_input_schema = Value::Object(search_tool.input_schema.as_ref().clone());
    assert_eq!(search_input_schema.get("type"), Some(&json!("object")));
    assert_additional_properties_false(&search_input_schema, "search input schema");
    assert_eq!(
        schema_property_names(&search_input_schema, "search input schema"),
        vec!["limit", "query"]
    );
    assert_eq!(
        schema_required_fields(&search_input_schema, "search input schema"),
        vec!["query"]
    );

    let search_query_schema = schema_property(&search_input_schema, "query", "search input schema");
    assert_eq!(search_query_schema.get("type"), Some(&json!("string")));
    assert_eq!(search_query_schema.get("minLength"), Some(&json!(1)));
    assert_eq!(search_query_schema.get("pattern"), Some(&json!(".*\\S.*")));

    let search_limit_schema = schema_property(&search_input_schema, "limit", "search input schema");
    assert_eq!(search_limit_schema.get("type"), Some(&json!("integer")));
    assert_eq!(search_limit_schema.get("minimum"), Some(&json!(1)));
    assert_eq!(search_limit_schema.get("maximum"), Some(&json!(10)));
    assert_eq!(search_limit_schema.get("default"), Some(&json!(5)));

    let search_output_root = Value::Object(
        search_tool
            .output_schema
            .as_ref()
            .expect("search output schema should exist")
            .as_ref()
            .clone(),
    );
    let search_output_schema = resolve_local_schema_ref(
        &search_output_root,
        &search_output_root,
        "search output root schema",
    );
    assert_eq!(search_output_schema.get("type"), Some(&json!("object")));
    assert_additional_properties_false(search_output_schema, "search output schema");
    assert_eq!(
        schema_property_names(search_output_schema, "search output schema"),
        vec!["results", "total_returned"]
    );
    assert_eq!(
        schema_required_fields(search_output_schema, "search output schema"),
        vec!["results", "total_returned"]
    );

    let search_results_schema =
        schema_property(search_output_schema, "results", "search output schema");
    assert_eq!(search_results_schema.get("type"), Some(&json!("array")));
    let search_result_card_schema = resolve_local_schema_ref(
        &search_output_root,
        search_results_schema
            .get("items")
            .expect("search results should include item schema"),
        "search result card schema",
    );
    assert_eq!(
        search_result_card_schema.get("type"),
        Some(&json!("object"))
    );
    assert_additional_properties_false(search_result_card_schema, "search result card schema");
    assert_eq!(
        schema_property_names(search_result_card_schema, "search result card schema"),
        vec!["snippet", "title", "url"]
    );
    assert_eq!(
        schema_required_fields(search_result_card_schema, "search result card schema"),
        vec!["title", "url"]
    );

    let summarize_tool = tools
        .iter()
        .find(|tool| tool.name == "kagi_summarize")
        .expect("kagi_summarize tool should exist");
    let summarize_input_schema = Value::Object(summarize_tool.input_schema.as_ref().clone());
    assert_eq!(summarize_input_schema.get("type"), Some(&json!("object")));
    assert_additional_properties_false(&summarize_input_schema, "summarize input schema");
    assert_eq!(
        schema_property_names(&summarize_input_schema, "summarize input schema"),
        vec!["text", "url"]
    );
    assert_eq!(
        summarize_input_schema.get("required"),
        None,
        "summarize input schema should not require top-level fields"
    );

    for forbidden_keyword in [
        "oneOf",
        "anyOf",
        "allOf",
        "not",
        "minProperties",
        "maxProperties",
    ] {
        assert_eq!(
            summarize_input_schema.get(forbidden_keyword),
            None,
            "summarize input schema must not publish `{forbidden_keyword}`"
        );
    }

    let summarize_url_property =
        schema_property(&summarize_input_schema, "url", "summarize input schema");
    assert_eq!(summarize_url_property.get("type"), Some(&json!("string")));
    for forbidden_keyword in ["format", "pattern", "minLength", "maxLength"] {
        assert_eq!(
            summarize_url_property.get(forbidden_keyword),
            None,
            "summarize url property must not publish `{forbidden_keyword}`"
        );
    }

    let summarize_text_property =
        schema_property(&summarize_input_schema, "text", "summarize input schema");
    assert_eq!(summarize_text_property.get("type"), Some(&json!("string")));
    for forbidden_keyword in ["format", "pattern", "minLength", "maxLength"] {
        assert_eq!(
            summarize_text_property.get(forbidden_keyword),
            None,
            "summarize text property must not publish `{forbidden_keyword}`"
        );
    }

    let summarize_output_root = Value::Object(
        summarize_tool
            .output_schema
            .as_ref()
            .expect("summarize output schema should exist")
            .as_ref()
            .clone(),
    );
    let summarize_output_schema = resolve_local_schema_ref(
        &summarize_output_root,
        &summarize_output_root,
        "summarize output root schema",
    );
    assert_eq!(summarize_output_schema.get("type"), Some(&json!("object")));
    assert_additional_properties_false(summarize_output_schema, "summarize output schema");
    assert_eq!(
        schema_property_names(summarize_output_schema, "summarize output schema"),
        vec!["markdown", "source_url", "text"]
    );
    assert_eq!(
        schema_required_fields(summarize_output_schema, "summarize output schema"),
        vec!["markdown"]
    );

    client
        .cancel()
        .await
        .expect("client shutdown should succeed");
    handle.await.expect("server join should succeed");
}

#[tokio::test]
async fn official_backend_parity_matrix_search_and_summarize_modes() {
    let mut server = Server::new();
    let _search_mock = server
        .mock("GET", "/api/v0/search")
        .match_query(Matcher::UrlEncoded("q".into(), "rust sdk".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"data":{"results":[{"title":"One","url":"https://one.test","snippet":"alpha","unused":"x"},{"title":"Two","url":"https://two.test"}]}}"#,
        )
        .create();

    let _summarize_get_mock = server
        .mock("GET", "/api/v0/summarize")
        .match_query(Matcher::UrlEncoded(
            "url".into(),
            "https://example.com/post".into(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "data": {
                    "markdown": "# Official URL",
                    "text": "url summary",
                    "source_url": "https://example.com/post",
                    "metadata": { "ignored": true }
                }
            })
            .to_string(),
        )
        .create();

    let _summarize_post_mock = server
        .mock("POST", "/api/v0/summarize")
        .match_body(Matcher::JsonString(
            json!({ "text": "  keep exact spacing  " }).to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "data": {
                    "markdown": "# Official Text",
                    "text": "  keep exact spacing  "
                }
            })
            .to_string(),
        )
        .create();

    let backend = build_official_backend(&server);
    let (client, handle) = start_server(backend).await;

    let search_result = client
        .call_tool(
            CallToolRequestParams::new("kagi_search").with_arguments(json_object(json!({
                "query": "  rust sdk  ",
                "limit": 2
            }))),
        )
        .await
        .expect("official search should succeed");
    assert_eq!(search_result.is_error, Some(false));
    let search_output: crate::SearchToolOutput = search_result
        .into_typed()
        .expect("search output should deserialize");
    assert_eq!(search_output.total_returned, 2);
    assert_eq!(search_output.results[0].title, "One");
    assert_eq!(search_output.results[1].url, "https://two.test");

    let summarize_url_result = client
        .call_tool(
            CallToolRequestParams::new("kagi_summarize").with_arguments(json_object(json!({
                "url": "https://example.com/post"
            }))),
        )
        .await
        .expect("official summarize(url) should succeed");
    let summarize_url_output: crate::SummarizeToolOutput = summarize_url_result
        .into_typed()
        .expect("summarize(url) output should deserialize");
    assert_eq!(summarize_url_output.markdown, "# Official URL");
    assert_eq!(
        summarize_url_output.source_url.as_deref(),
        Some("https://example.com/post")
    );

    let summarize_text_result = client
        .call_tool(
            CallToolRequestParams::new("kagi_summarize").with_arguments(json_object(json!({
                "text": "  keep exact spacing  "
            }))),
        )
        .await
        .expect("official summarize(text) should succeed");
    let summarize_text_output: crate::SummarizeToolOutput = summarize_text_result
        .into_typed()
        .expect("summarize(text) output should deserialize");
    assert_eq!(summarize_text_output.markdown, "# Official Text");
    assert_eq!(
        summarize_text_output.text.as_deref(),
        Some("  keep exact spacing  ")
    );

    client
        .cancel()
        .await
        .expect("client shutdown should succeed");
    handle.await.expect("server join should succeed");
}

#[tokio::test]
async fn session_backend_parity_matrix_search_and_summarize_modes() {
    let mut server = Server::new();
    let _search_mock = server
        .mock("GET", "/html/search")
        .match_query(Matcher::UrlEncoded("q".into(), "rust sdk".into()))
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(search_result_html(3))
        .create();

    let _summarize_url_mock = server
        .mock("GET", "/mother/summary_labs")
        .match_query(Matcher::UrlEncoded(
            "url".into(),
            "https://example.com/post".into(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "markdown": "# Session URL",
                "text": "session url",
                "metadata": {
                    "source_url": "https://example.com/post",
                    "tokens": 42
                }
            })
            .to_string(),
        )
        .create();

    let _summarize_text_mock = server
        .mock("POST", "/mother/summary_labs/")
        .match_body(Matcher::UrlEncoded(
            "text".to_string(),
            "  keep exact spacing  ".to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "markdown": "# Session Text",
                "text": "  keep exact spacing  "
            })
            .to_string(),
        )
        .create();

    let backend = build_session_backend(&server);
    let (client, handle) = start_server(backend).await;

    let search_result = client
        .call_tool(
            CallToolRequestParams::new("kagi_search").with_arguments(json_object(json!({
                "query": "  rust sdk  ",
                "limit": 2
            }))),
        )
        .await
        .expect("session search should succeed");
    let search_output: crate::SearchToolOutput = search_result
        .into_typed()
        .expect("search output should deserialize");
    assert_eq!(search_output.total_returned, 2);
    assert_eq!(search_output.results[0].title, "Result 0");

    let summarize_url_result = client
        .call_tool(
            CallToolRequestParams::new("kagi_summarize").with_arguments(json_object(json!({
                "url": "https://example.com/post"
            }))),
        )
        .await
        .expect("session summarize(url) should succeed");
    let summarize_url_output: crate::SummarizeToolOutput = summarize_url_result
        .into_typed()
        .expect("summarize(url) output should deserialize");
    assert_eq!(summarize_url_output.markdown, "# Session URL");
    assert_eq!(
        summarize_url_output.source_url.as_deref(),
        Some("https://example.com/post")
    );

    let summarize_text_result = client
        .call_tool(
            CallToolRequestParams::new("kagi_summarize").with_arguments(json_object(json!({
                "text": "  keep exact spacing  "
            }))),
        )
        .await
        .expect("session summarize(text) should succeed");
    let summarize_text_output: crate::SummarizeToolOutput = summarize_text_result
        .into_typed()
        .expect("summarize(text) output should deserialize");
    assert_eq!(summarize_text_output.markdown, "# Session Text");
    assert_eq!(
        summarize_text_output.text.as_deref(),
        Some("  keep exact spacing  ")
    );

    client
        .cancel()
        .await
        .expect("client shutdown should succeed");
    handle.await.expect("server join should succeed");
}

#[tokio::test]
async fn search_validation_rules_and_limit_behavior_are_enforced() {
    let mut server = Server::new();
    let _search_mock = server
        .mock("GET", "/api/v0/search")
        .match_query(Matcher::UrlEncoded("q".into(), "rust".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"data":{"results":[{"title":"1","url":"https://1.test"},{"title":"2","url":"https://2.test"},{"title":"3","url":"https://3.test"},{"title":"4","url":"https://4.test"},{"title":"5","url":"https://5.test"},{"title":"6","url":"https://6.test"}]}}"#,
        )
        .create();

    let backend = build_official_backend(&server);
    let (client, handle) = start_server(backend).await;

    let missing_query = client
        .call_tool(
            CallToolRequestParams::new("kagi_search").with_arguments(json_object(json!({
                "limit": 3
            }))),
        )
        .await
        .expect_err("missing query must fail");
    assert_invalid_params(missing_query);

    let blank_query = client
        .call_tool(
            CallToolRequestParams::new("kagi_search").with_arguments(json_object(json!({
                "query": "    "
            }))),
        )
        .await
        .expect_err("blank query must fail");
    assert_invalid_params(blank_query);

    let invalid_limit_low = client
        .call_tool(
            CallToolRequestParams::new("kagi_search").with_arguments(json_object(json!({
                "query": "rust",
                "limit": 0
            }))),
        )
        .await
        .expect_err("limit below min must fail");
    assert_invalid_params(invalid_limit_low);

    let invalid_limit_high = client
        .call_tool(
            CallToolRequestParams::new("kagi_search").with_arguments(json_object(json!({
                "query": "rust",
                "limit": 11
            }))),
        )
        .await
        .expect_err("limit above max must fail");
    assert_invalid_params(invalid_limit_high);

    let unknown_field = client
        .call_tool(
            CallToolRequestParams::new("kagi_search").with_arguments(json_object(json!({
                "query": "rust",
                "extra": true
            }))),
        )
        .await
        .expect_err("unknown field must fail");
    assert_invalid_params(unknown_field);

    let default_limit_result = client
        .call_tool(
            CallToolRequestParams::new("kagi_search").with_arguments(json_object(json!({
                "query": "  rust  "
            }))),
        )
        .await
        .expect("search with default limit should succeed");
    let default_limit_output: crate::SearchToolOutput = default_limit_result
        .into_typed()
        .expect("default limit output should deserialize");
    assert_eq!(default_limit_output.total_returned, 5);

    let explicit_limit_result = client
        .call_tool(
            CallToolRequestParams::new("kagi_search").with_arguments(json_object(json!({
                "query": "rust",
                "limit": 1
            }))),
        )
        .await
        .expect("search with explicit limit should succeed");
    let explicit_limit_output: crate::SearchToolOutput = explicit_limit_result
        .into_typed()
        .expect("explicit limit output should deserialize");
    assert_eq!(explicit_limit_output.total_returned, 1);

    client
        .cancel()
        .await
        .expect("client shutdown should succeed");
    handle.await.expect("server join should succeed");
}

#[tokio::test]
async fn summarize_validation_rules_and_byte_limits_are_enforced() {
    let mut server = Server::new();
    let _summarize_url_mock = server
        .mock("GET", "/api/v0/summarize")
        .match_query(Matcher::UrlEncoded(
            "url".into(),
            "https://example.com/post".into(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(json!({ "data": { "markdown": "# URL" } }).to_string())
        .create();

    let _summarize_text_mock = server
        .mock("POST", "/api/v0/summarize")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(json!({ "data": { "markdown": "# TEXT" } }).to_string())
        .create();

    let backend = build_official_backend(&server);
    let (client, handle) = start_server(backend).await;

    let missing_both = client
        .call_tool(
            CallToolRequestParams::new("kagi_summarize").with_arguments(json_object(json!({}))),
        )
        .await
        .expect_err("missing both url/text must fail");
    assert_invalid_params(missing_both);

    let url_with_empty_text = client
        .call_tool(
            CallToolRequestParams::new("kagi_summarize").with_arguments(json_object(json!({
                "url": "https://example.com/post",
                "text": ""
            }))),
        )
        .await
        .expect("url should succeed when text is exact empty string");
    let url_with_empty_text_output: crate::SummarizeToolOutput = url_with_empty_text
        .into_typed()
        .expect("url+empty text output should deserialize");
    assert_eq!(url_with_empty_text_output.markdown, "# URL");

    let text_with_empty_url = client
        .call_tool(
            CallToolRequestParams::new("kagi_summarize").with_arguments(json_object(json!({
                "text": "real text",
                "url": ""
            }))),
        )
        .await
        .expect("text should succeed when url is exact empty string");
    let text_with_empty_url_output: crate::SummarizeToolOutput = text_with_empty_url
        .into_typed()
        .expect("text+empty url output should deserialize");
    assert_eq!(text_with_empty_url_output.markdown, "# TEXT");

    let both_fields = client
        .call_tool(
            CallToolRequestParams::new("kagi_summarize").with_arguments(json_object(json!({
                "url": "https://example.com/post",
                "text": "hello"
            }))),
        )
        .await
        .expect_err("url+text together must fail");
    assert_invalid_params(both_fields);

    let both_empty_fields = client
        .call_tool(
            CallToolRequestParams::new("kagi_summarize").with_arguments(json_object(json!({
                "url": "",
                "text": ""
            }))),
        )
        .await
        .expect_err("url+text exact empty strings must fail");
    assert_invalid_params(both_empty_fields);

    let url_with_whitespace_only_text = client
        .call_tool(
            CallToolRequestParams::new("kagi_summarize").with_arguments(json_object(json!({
                "url": "https://example.com/post",
                "text": "   "
            }))),
        )
        .await
        .expect_err("url + whitespace-only text must fail");
    assert_invalid_params(url_with_whitespace_only_text);

    let text_with_whitespace_padded_url = client
        .call_tool(
            CallToolRequestParams::new("kagi_summarize").with_arguments(json_object(json!({
                "text": "real text",
                "url": " https://example.com/post"
            }))),
        )
        .await
        .expect_err("text + whitespace-padded url must fail");
    assert_invalid_params(text_with_whitespace_padded_url);

    let text_with_whitespace_only_url = client
        .call_tool(
            CallToolRequestParams::new("kagi_summarize").with_arguments(json_object(json!({
                "text": "real text",
                "url": "   "
            }))),
        )
        .await
        .expect_err("text + whitespace-only url must fail");
    assert_invalid_params(text_with_whitespace_only_url);

    let invalid_url = client
        .call_tool(
            CallToolRequestParams::new("kagi_summarize").with_arguments(json_object(json!({
                "url": "ftp://example.com/post"
            }))),
        )
        .await
        .expect_err("non-http url must fail");
    assert_invalid_params(invalid_url);

    let padded_url = client
        .call_tool(
            CallToolRequestParams::new("kagi_summarize").with_arguments(json_object(json!({
                "url": " https://example.com/post "
            }))),
        )
        .await
        .expect_err("whitespace-padded url must fail");
    assert_invalid_params(padded_url);

    let blank_text = client
        .call_tool(
            CallToolRequestParams::new("kagi_summarize").with_arguments(json_object(json!({
                "text": "    "
            }))),
        )
        .await
        .expect_err("blank text must fail");
    assert_invalid_params(blank_text);

    let unknown_field = client
        .call_tool(
            CallToolRequestParams::new("kagi_summarize").with_arguments(json_object(json!({
                "url": "https://example.com/post",
                "backend": "official"
            }))),
        )
        .await
        .expect_err("unknown summarize field must fail");
    assert_invalid_params(unknown_field);

    let accepted_text = "a".repeat(50_000);
    client
        .call_tool(
            CallToolRequestParams::new("kagi_summarize").with_arguments(json_object(json!({
                "text": accepted_text
            }))),
        )
        .await
        .expect("50_000 byte summarize text should be accepted");

    let rejected_text = "a".repeat(50_001);
    let rejected_result = client
        .call_tool(
            CallToolRequestParams::new("kagi_summarize").with_arguments(json_object(json!({
                "text": rejected_text
            }))),
        )
        .await
        .expect_err("50_001 byte summarize text should be rejected");
    assert_invalid_params(rejected_result);

    let accepted_multibyte_text = "é".repeat(25_000);
    client
        .call_tool(
            CallToolRequestParams::new("kagi_summarize").with_arguments(json_object(json!({
                "text": accepted_multibyte_text
            }))),
        )
        .await
        .expect("50_000-byte multibyte summarize text should be accepted");

    let rejected_multibyte_text = "é".repeat(25_001);
    let rejected_multibyte_result = client
        .call_tool(
            CallToolRequestParams::new("kagi_summarize").with_arguments(json_object(json!({
                "text": rejected_multibyte_text
            }))),
        )
        .await
        .expect_err("50_002-byte multibyte summarize text should be rejected");
    assert_invalid_params(rejected_multibyte_result);

    client
        .cancel()
        .await
        .expect("client shutdown should succeed");
    handle.await.expect("server join should succeed");
}

#[tokio::test]
async fn outputs_never_leak_provider_envelopes_or_metadata_and_do_not_truncate() {
    let mut server = Server::new();
    let long_markdown = "x".repeat(70_000);

    let _search_mock = server
        .mock("GET", "/api/v0/search")
        .match_query(Matcher::UrlEncoded("q".into(), "rust".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"data":{"results":[{"title":"One","url":"https://one.test","snippet":"alpha","provider_meta":{"rank":1}}],"extra":{"ignored":true}}}"#,
        )
        .create();

    let _summarize_mock = server
        .mock("GET", "/api/v0/summarize")
        .match_query(Matcher::UrlEncoded(
            "url".into(),
            "https://example.com/post".into(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "data": {
                    "markdown": long_markdown,
                    "text": "plain",
                    "metadata": {"tokens": 999},
                    "status": "ok"
                }
            })
            .to_string(),
        )
        .create();

    let backend = build_official_backend(&server);
    let (client, handle) = start_server(backend).await;

    let search_result = client
        .call_tool(
            CallToolRequestParams::new("kagi_search").with_arguments(json_object(json!({
                "query": "rust",
                "limit": 5
            }))),
        )
        .await
        .expect("search should succeed");
    let search_output: crate::SearchToolOutput = search_result
        .into_typed()
        .expect("search output should deserialize");
    let serialized_search =
        serde_json::to_value(search_output).expect("search output should serialize");
    let mut search_keys = serialized_search
        .as_object()
        .expect("search output should be object")
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    search_keys.sort();
    assert_eq!(search_keys, vec!["results", "total_returned"]);

    let summarize_result = client
        .call_tool(
            CallToolRequestParams::new("kagi_summarize").with_arguments(json_object(json!({
                "url": "https://example.com/post"
            }))),
        )
        .await
        .expect("summarize should succeed");
    let summarize_output: crate::SummarizeToolOutput = summarize_result
        .into_typed()
        .expect("summarize output should deserialize");
    assert_eq!(summarize_output.markdown.len(), 70_000);
    let serialized_summarize =
        serde_json::to_value(summarize_output).expect("summarize output should serialize");
    let mut summarize_keys = serialized_summarize
        .as_object()
        .expect("summarize output should be object")
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    summarize_keys.sort();
    assert_eq!(summarize_keys, vec!["markdown", "source_url", "text"]);

    client
        .cancel()
        .await
        .expect("client shutdown should succeed");
    handle.await.expect("server join should succeed");
}

#[tokio::test]
async fn tool_errors_are_mapped_for_auth_upstream_parse_and_transport_failures() {
    let mut auth_server = Server::new();
    let _auth_mock = auth_server
        .mock("GET", "/api/v0/search")
        .match_query(Matcher::UrlEncoded("q".into(), "rust".into()))
        .with_status(401)
        .with_body("bad token secret_auth_token")
        .create();

    let auth_backend = build_official_backend(&auth_server);
    let (auth_client, auth_handle) = start_server(auth_backend).await;
    let auth_result = auth_client
        .call_tool(
            CallToolRequestParams::new("kagi_search").with_arguments(json_object(json!({
                "query": "rust"
            }))),
        )
        .await
        .expect("auth failure should still return tool result");
    assert_eq!(auth_result.is_error, Some(true));
    let auth_text = auth_result
        .content
        .first()
        .and_then(|content| content.as_text())
        .map(|content| content.text.clone())
        .expect("auth failure should return text error");
    assert!(auth_text.contains("Authentication failed"));
    assert!(auth_text.contains(ENV_API_KEY));
    assert!(auth_text.contains(ENV_SESSION_TOKEN));
    assert!(!auth_text.contains("secret_auth_token"));
    auth_client
        .cancel()
        .await
        .expect("client shutdown should succeed");
    auth_handle.await.expect("server join should succeed");

    let mut upstream_server = Server::new();
    let _upstream_mock = upstream_server
        .mock("GET", "/api/v0/search")
        .match_query(Matcher::UrlEncoded("q".into(), "rust".into()))
        .with_status(429)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error":"rate limited"}"#)
        .create();

    let upstream_backend = build_official_backend(&upstream_server);
    let (upstream_client, upstream_handle) = start_server(upstream_backend).await;
    let upstream_result = upstream_client
        .call_tool(
            CallToolRequestParams::new("kagi_search").with_arguments(json_object(json!({
                "query": "rust"
            }))),
        )
        .await
        .expect("upstream failure should still return tool result");
    let upstream_text = upstream_result
        .content
        .first()
        .and_then(|content| content.as_text())
        .map(|content| content.text.clone())
        .expect("upstream failure should return text error");
    assert!(upstream_text.contains("HTTP 429"));
    upstream_client
        .cancel()
        .await
        .expect("client shutdown should succeed");
    upstream_handle.await.expect("server join should succeed");

    let mut parse_server = Server::new();
    let _parse_mock = parse_server
        .mock("GET", "/api/v0/search")
        .match_query(Matcher::UrlEncoded("q".into(), "rust".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"data":{"unexpected":[]}}"#)
        .create();

    let parse_backend = build_official_backend(&parse_server);
    let (parse_client, parse_handle) = start_server(parse_backend).await;
    let parse_result = parse_client
        .call_tool(
            CallToolRequestParams::new("kagi_search").with_arguments(json_object(json!({
                "query": "rust"
            }))),
        )
        .await
        .expect("parse drift should still return tool result");
    let parse_text = parse_result
        .content
        .first()
        .and_then(|content| content.as_text())
        .map(|content| content.text.clone())
        .expect("parse drift should return text error");
    assert!(parse_text.contains("unexpected response shape"));
    parse_client
        .cancel()
        .await
        .expect("client shutdown should succeed");
    parse_handle.await.expect("server join should succeed");

    let transport_backend = BackendRuntime::from_env_config(
        EnvConfig {
            backend_mode: Some("official".to_string()),
            api_key: Some("official_token".to_string()),
            session_token: None,
        },
        ClientConfig::default()
            .with_base_url(Url::parse("http://127.0.0.1:9").expect("url should parse"))
            .with_timeout(Duration::from_millis(50)),
    )
    .expect("transport backend should build");

    let (transport_client, transport_handle) = start_server(transport_backend).await;
    let transport_result = transport_client
        .call_tool(
            CallToolRequestParams::new("kagi_search").with_arguments(json_object(json!({
                "query": "rust"
            }))),
        )
        .await
        .expect("transport failure should still return tool result");
    let transport_text = transport_result
        .content
        .first()
        .and_then(|content| content.as_text())
        .map(|content| content.text.clone())
        .expect("transport failure should return text error");
    assert!(transport_text.contains("transport") || transport_text.contains("timed out"));

    transport_client
        .cancel()
        .await
        .expect("client shutdown should succeed");
    transport_handle.await.expect("server join should succeed");
}

#[tokio::test]
async fn session_summarize_http_200_auth_like_failures_map_to_auth_message() {
    for unauthorized_payload in [
        r#"{"error":"Unauthorized"}"#,
        r#"{"success":false,"message":"Unauthorized: Unauthorized"}"#,
    ] {
        let mut server = Server::new();
        let _summarize_mock = server
            .mock("GET", "/mother/summary_labs")
            .match_query(Matcher::UrlEncoded(
                "url".into(),
                "https://example.com/protected".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(unauthorized_payload)
            .create();

        let backend = build_session_backend(&server);
        let (client, handle) = start_server(backend).await;
        let summarize_result = client
            .call_tool(
                CallToolRequestParams::new("kagi_summarize").with_arguments(json_object(json!({
                    "url": "https://example.com/protected"
                }))),
            )
            .await
            .expect("auth-like summarize failure should still return tool result");

        assert_eq!(summarize_result.is_error, Some(true));
        let summarize_error_text = summarize_result
            .content
            .first()
            .and_then(|content| content.as_text())
            .map(|content| content.text.clone())
            .expect("auth-like summarize failure should return text error");

        assert!(summarize_error_text.contains("Authentication failed with Kagi"));
        assert!(summarize_error_text.contains(ENV_SESSION_TOKEN));
        assert!(summarize_error_text.contains(ENV_API_KEY));
        assert!(summarize_error_text.contains("may belong"));
        assert!(!summarize_error_text.contains("application-level failure"));

        client
            .cancel()
            .await
            .expect("client shutdown should succeed");
        handle.await.expect("server join should succeed");
    }
}

#[test]
fn startup_backend_selection_and_failures_match_contract() {
    let default_config = ClientConfig::default();

    let auto_prefers_official = BackendRuntime::from_env_config(
        EnvConfig {
            backend_mode: Some("auto".to_string()),
            api_key: Some("official_token".to_string()),
            session_token: Some("session_token".to_string()),
        },
        default_config.clone(),
    )
    .expect("auto mode should prefer api key");
    assert!(matches!(auto_prefers_official, BackendRuntime::Official(_)));

    let auto_falls_back_to_session = BackendRuntime::from_env_config(
        EnvConfig {
            backend_mode: Some("auto".to_string()),
            api_key: None,
            session_token: Some("session_token".to_string()),
        },
        default_config.clone(),
    )
    .expect("auto mode should use session when api key missing");
    assert!(matches!(
        auto_falls_back_to_session,
        BackendRuntime::Session(_)
    ));

    let missing = BackendRuntime::from_env_config(
        EnvConfig {
            backend_mode: Some("auto".to_string()),
            api_key: None,
            session_token: None,
        },
        default_config.clone(),
    )
    .expect_err("auto mode without any credential must fail");
    assert!(missing.to_string().contains(ENV_API_KEY));

    let missing_official_with_session_present = BackendRuntime::from_env_config(
        EnvConfig {
            backend_mode: Some("official".to_string()),
            api_key: None,
            session_token: Some("session_token".to_string()),
        },
        default_config.clone(),
    )
    .expect_err("official mode without api key must fail");
    let missing_official_message = missing_official_with_session_present.to_string();
    assert!(missing_official_message.contains(ENV_API_KEY));
    assert!(missing_official_message.contains(ENV_SESSION_TOKEN));
    assert!(missing_official_message.contains("may belong"));
    assert!(missing_official_message.contains("session"));

    let missing_session_with_api_present = BackendRuntime::from_env_config(
        EnvConfig {
            backend_mode: Some("session".to_string()),
            api_key: Some("official_token".to_string()),
            session_token: None,
        },
        default_config.clone(),
    )
    .expect_err("session mode without session token must fail");
    let missing_session_message = missing_session_with_api_present.to_string();
    assert!(missing_session_message.contains(ENV_SESSION_TOKEN));
    assert!(missing_session_message.contains(ENV_API_KEY));
    assert!(missing_session_message.contains("may belong"));
    assert!(missing_session_message.contains("official"));

    let invalid_mode = BackendRuntime::from_env_config(
        EnvConfig {
            backend_mode: Some("invalid".to_string()),
            api_key: Some("official_token".to_string()),
            session_token: None,
        },
        default_config.clone(),
    )
    .expect_err("invalid backend mode must fail");
    assert!(invalid_mode.to_string().contains(ENV_BACKEND_MODE));

    let invalid_official_credential = BackendRuntime::from_env_config(
        EnvConfig {
            backend_mode: Some("official".to_string()),
            api_key: Some("secret official token".to_string()),
            session_token: None,
        },
        default_config.clone(),
    )
    .expect_err("invalid official credential must fail");
    let invalid_official_message = invalid_official_credential.to_string();
    assert!(invalid_official_message.contains(ENV_API_KEY));
    assert!(!invalid_official_message.contains("secret official token"));

    let invalid_session_credential = BackendRuntime::from_env_config(
        EnvConfig {
            backend_mode: Some("session".to_string()),
            api_key: None,
            session_token: Some("secret session token".to_string()),
        },
        default_config,
    )
    .expect_err("invalid session credential must fail");
    let invalid_session_message = invalid_session_credential.to_string();
    assert!(invalid_session_message.contains(ENV_SESSION_TOKEN));
    assert!(!invalid_session_message.contains("secret session token"));
}
