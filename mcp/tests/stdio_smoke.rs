use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

#[tokio::test]
async fn stdio_smoke_outputs_only_json_rpc_messages_on_stdout() {
    let binary = env!("CARGO_BIN_EXE_kagi-mcp");
    let mut child = Command::new(binary)
        .env("KAGI_MCP_BACKEND", "official")
        .env("KAGI_API_KEY", "official_token")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to launch kagi-mcp binary");

    let mut stdin = child.stdin.take().expect("stdin should be piped");
    let stdout = child.stdout.take().expect("stdout should be piped");
    let mut lines = BufReader::new(stdout).lines();

    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"stdio-smoke","version":"0.1.0"}}}
"#,
        )
        .await
        .expect("initialize request should write");

    let initialize_line = timeout(Duration::from_secs(5), lines.next_line())
        .await
        .expect("initialize response timeout")
        .expect("initialize response read should succeed")
        .expect("initialize response should be present");

    let initialize_json: serde_json::Value =
        serde_json::from_str(&initialize_line).expect("initialize stdout must be valid json-rpc");
    assert_eq!(initialize_json["id"], 1);

    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","method":"notifications/initialized"}
"#,
        )
        .await
        .expect("initialized notification should write");

    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}
"#,
        )
        .await
        .expect("tools/list request should write");

    let tools_line = timeout(Duration::from_secs(5), lines.next_line())
        .await
        .expect("tools/list response timeout")
        .expect("tools/list response read should succeed")
        .expect("tools/list response should be present");

    let tools_json: serde_json::Value =
        serde_json::from_str(&tools_line).expect("tools/list stdout must be valid json-rpc");
    assert_eq!(tools_json["id"], 2);
    assert_eq!(
        tools_json["result"]["tools"]
            .as_array()
            .expect("tools must be an array")
            .len(),
        2
    );

    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"kagi_summarize","arguments":{"url":"https://example.com/post","text":"hello"}}}
"#,
        )
        .await
        .expect("tools/call invalid-params request should write");

    let summarize_error_line = timeout(Duration::from_secs(5), lines.next_line())
        .await
        .expect("tools/call invalid-params response timeout")
        .expect("tools/call invalid-params response read should succeed")
        .expect("tools/call invalid-params response should be present");

    let summarize_error_json: serde_json::Value = serde_json::from_str(&summarize_error_line)
        .expect("tools/call invalid-params stdout must be valid json-rpc");
    assert_eq!(summarize_error_json["id"], 3);
    assert!(summarize_error_json.get("error").is_some());

    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"kagi_summarize","arguments":{"url":" https://example.com/post "}}}
"#,
        )
        .await
        .expect("tools/call whitespace-url request should write");

    let whitespace_url_error_line = timeout(Duration::from_secs(5), lines.next_line())
        .await
        .expect("tools/call whitespace-url response timeout")
        .expect("tools/call whitespace-url response read should succeed")
        .expect("tools/call whitespace-url response should be present");

    let whitespace_url_error_json: serde_json::Value =
        serde_json::from_str(&whitespace_url_error_line)
            .expect("tools/call whitespace-url stdout must be valid json-rpc");
    assert_eq!(whitespace_url_error_json["id"], 4);
    assert!(whitespace_url_error_json.get("error").is_some());

    drop(stdin);
    let exit_status = timeout(Duration::from_secs(5), child.wait())
        .await
        .expect("child process did not exit after stdin closed")
        .expect("failed waiting for child process");
    assert!(exit_status.success());
}
