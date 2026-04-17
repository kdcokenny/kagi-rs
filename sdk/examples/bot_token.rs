use kagi_sdk::{
    official_api::models::{SearchRequest, SmallwebFeedRequest},
    BotToken, KagiClient,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = KagiClient::with_bot_token(BotToken::new("kagi_bot_token_here")?)?;
    let official_api = client.official_api()?;

    let _search = SearchRequest::new("rust sdk design")?;
    let _feed = SmallwebFeedRequest::with_limit(10)?;

    let _ = official_api;
    Ok(())
}
