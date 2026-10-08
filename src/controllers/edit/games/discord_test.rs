use super::*;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestDiscordWebhookModel {
    #[serde(default)]
    pub webhook_url: Option<String>,
}

/// `POST /api/edit/games/{id}/discord/test`
///
/// Sends an immediate test delivery using either the draft webhook URL from
/// the request body or the persisted webhook URL for the game.
pub async fn test_discord_webhook(
    State(st): State<SharedState>,
    _admin: AdminUser,
    Path(id): Path<i32>,
    Json(model): Json<TestDiscordWebhookModel>,
) -> AppResult<MessageResponse> {
    let stored_webhook: Option<Option<String>> = sqlx::query_scalar(
        r#"SELECT discord_webhook FROM "Games" WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(st.pg())
    .await
    .map_err(|error| AppError::internal(error.to_string()))?;

    let stored_webhook = match stored_webhook {
        Some(webhook) => webhook,
        None => return Err(AppError::not_found("Game not found")),
    };

    let target_url = model
        .webhook_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| stored_webhook.as_deref())
        .ok_or_else(|| AppError::bad_request("No Discord webhook URL provided"))?;

    crate::services::discord_webhook::send_test_discord_webhook(target_url).await?;

    Ok(MessageResponse::ok("Test notification sent successfully"))
}
