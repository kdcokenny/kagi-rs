use kagi_sdk::{
    session_web::models::{SearchRequest, SummarizeRequest, SummaryType},
    KagiClient, SessionToken,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = KagiClient::with_session_token(SessionToken::new("kagi_session_token_here")?)?;
    let session_web = client.session_web()?;

    let _search = SearchRequest::new("kagi session web")?;
    let _summarize = SummarizeRequest::from_url("https://example.com/article")?
        .with_summary_type(SummaryType::Summary)
        .with_target_language("en")?;
    let _summarize_stream = SummarizeRequest::from_text("Summarize this text")?;

    let _ = session_web;
    Ok(())
}
