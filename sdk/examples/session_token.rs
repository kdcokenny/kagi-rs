use kagi_sdk::{
    session_web::models::{HtmlSearchRequest, SummaryLabsTextRequest, SummaryLabsUrlRequest},
    KagiClient, SessionToken,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = KagiClient::with_session_token(SessionToken::new("kagi_session_token_here")?)?;
    let session_web = client.session_web()?;

    let _html_search = HtmlSearchRequest::new("kagi session web")?;
    let _summary_url = SummaryLabsUrlRequest::new("https://example.com/article")?;
    let _summary_text = SummaryLabsTextRequest::new("Summarize this text")?;

    let _ = session_web;
    Ok(())
}
