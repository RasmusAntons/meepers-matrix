use matrix_sdk::ruma::UserId;
use matrix_sdk::{Client, config::SyncSettings};
use std::env;

mod abilities;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let meepers = UserId::parse(env::var("BOT_USER_ID").unwrap_or("@meepers:enigmatics.org".to_string()))
        .expect("invalid BOT_USER_ID");
    let client = Client::builder().server_name(meepers.server_name()).build().await?;
    client
        .matrix_auth()
        .login_username(
            &meepers,
            env::var("BOT_PASSWORD").expect("missing BOT_PASSWORD").as_str(),
        )
        .initial_device_display_name(meepers.localpart())
        .send()
        .await?;
    let sync_token = client.sync_once(SyncSettings::default()).await?.next_batch;
    client.add_event_handler(abilities::on_message);
    client.sync(SyncSettings::default().token(sync_token)).await?;
    Ok(())
}
