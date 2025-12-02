use crate::AppState;
use crate::arcanite::{UpdateChannel};
use anyhow::{Context, bail};
use chrono::{DateTime, Utc};
use twilight_http::client::ClientBuilder;
use twilight_model::util::Timestamp;
use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder};
use twilight_util::link::webhook;

pub async fn send_webhook(
    state: &AppState,
    channel: &UpdateChannel,
    launcher_version: &str,
    java_version: &str,
    game_version: &str,
    game_build_number: u64,
    timestamp: DateTime<Utc>
) -> anyhow::Result<()> {
    let Some(webhook_url) = state.discord_webhook_url.clone() else {
        bail!("No webhook url provided!");
    };
    let (id, token) = webhook::parse(&webhook_url)
        .with_context(|| format!("Failed to parse webhook URL: {webhook_url}"))?;
    let token = token.with_context(|| format!("webhook URL contained no token: {webhook_url}"))?;

    let client = ClientBuilder::new().build();

    let embed = EmbedBuilder::new()
        .color(channel.color())
        .title("New Game Build")
        .timestamp(
            Timestamp::from_secs(timestamp.timestamp())
                .expect("Tried to construct invalid timestamp!"),
        )
        .field(EmbedFieldBuilder::new(
            "Launcher Version",
            launcher_version,
        ))
        .field(EmbedFieldBuilder::new(
            "Java Version",
            java_version,
        ))
        .field(EmbedFieldBuilder::new("Game Version", game_version).inline())
        .field(
            EmbedFieldBuilder::new("Build Number", format!("{game_build_number}"))
                .inline(),
        );

    let embeds = vec![embed.build()];
    let response = client
        .execute_webhook(id, token)
        // TODO beautify webhook
        // .username()
        // .avatar_url()
        .embeds(&embeds)
        .await?;

    if !response.status().is_success() {
        log::error!(
            "Received error status from webhook: {status}",
            status = response.status()
        )
    }

    Ok(())
}
