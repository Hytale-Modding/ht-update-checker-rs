use crate::AppState;
use crate::arcanite::{JavaVersionMeta, LauncherMeta, Patchline};
use anyhow::{Context, bail};
use chrono::Utc;
use twilight_http::client::ClientBuilder;
use twilight_model::util::Timestamp;
use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder};
use twilight_util::link::webhook;

pub async fn send_webhook(
    state: &AppState,
    launcher_meta: &LauncherMeta,
    java_version_meta: &JavaVersionMeta,
    patch_info: &Patchline,
) -> anyhow::Result<()> {
    let Some(webhook_url) = state.discord_webhook_url.clone() else {
        bail!("No webhook url provided!");
    };
    let (id, token) = webhook::parse(&webhook_url)
        .with_context(|| format!("Failed to parse webhook URL: {webhook_url}"))?;
    let token = token.with_context(|| format!("webhook URL contained no token: {webhook_url}"))?;

    let client = ClientBuilder::new().build();

    let embed = EmbedBuilder::new()
        .color(0x476ed1)
        .title("New Game Build")
        .timestamp(
            Timestamp::from_secs(Utc::now().timestamp())
                .expect("Tried to construct invalid timestamp!"),
        )
        .field(EmbedFieldBuilder::new(
            "Launcher Version",
            &launcher_meta.version,
        ))
        .field(EmbedFieldBuilder::new(
            "Java Version",
            &java_version_meta.version,
        ))
        .field(EmbedFieldBuilder::new("Game Version", &patch_info.build_version).inline())
        .field(
            EmbedFieldBuilder::new("Build Number", format!("{}", &patch_info.current_build))
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
