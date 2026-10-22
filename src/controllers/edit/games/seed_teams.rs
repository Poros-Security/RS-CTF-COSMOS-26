//! Batch seeding of empty, claimable teams for an event.

use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::app_state::SharedState;
use crate::middlewares::privilege_authentication::CurrentUser;
use crate::utils::codec::random_hex;
use crate::utils::error::{AppError, AppResult};
use crate::utils::shared::RequestResponse;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSeedTeamsModel {
    pub team_names: Vec<String>,
    #[serde(default)]
    pub division_id: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeededTeamResult {
    pub team_id: i32,
    pub team_name: String,
    pub invite_code: String,
}

/// `POST /api/edit/games/{id}/seed-teams`
///
/// Batch create empty teams without a captain and enroll them into the event.
/// Each team is created with `captain_id = NULL` and a unique `invite_token`.
/// The first member who joins via `POST /api/team/accept` automatically
/// becomes the captain.
pub async fn seed_teams(
    State(st): State<SharedState>,
    user: CurrentUser,
    Path(id): Path<i32>,
    Json(model): Json<GameSeedTeamsModel>,
) -> AppResult<RequestResponse<Vec<SeededTeamResult>>> {
    super::manager_or_admin(&st, &user, id).await?;
    let game = crate::controllers::edit::helpers::load_game(&st, id).await?;

    if model.team_names.is_empty() {
        return Err(AppError::bad_request("At least one team name is required"));
    }
    if model.team_names.len() > 100 {
        return Err(AppError::payload_too_large(
            "Batch team creation accepts up to 100 teams per request",
        ));
    }

    // Validate division if provided
    if let Some(div_id) = model.division_id {
        let div_exists: bool = sqlx::query_scalar(
            r#"SELECT EXISTS(SELECT 1 FROM "Divisions" WHERE id = $1 AND game_id = $2)"#,
        )
        .bind(div_id)
        .bind(id)
        .fetch_one(st.pg())
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
        if !div_exists {
            return Err(AppError::bad_request("Invalid division for this game"));
        }
    }

    // Validate team names
    let mut cleaned_names = Vec::with_capacity(model.team_names.len());
    let mut seen = std::collections::HashSet::new();
    for name in model.team_names {
        let trimmed = name.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        crate::controllers::team::validate_team_profile(Some(&trimmed), None)?;
        if !seen.insert(trimmed.to_uppercase()) {
            return Err(AppError::bad_request(format!(
                "Duplicate team name in batch: {trimmed}"
            )));
        }
        cleaned_names.push(trimmed);
    }

    if cleaned_names.is_empty() {
        return Err(AppError::bad_request("No valid team names provided"));
    }

    let mut transaction = st
        .pg()
        .begin()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    let mut results = Vec::with_capacity(cleaned_names.len());

    for team_name in cleaned_names {
        // Fetch existing team or insert a new general platform team
        let existing_team: Option<(i32, String)> =
            sqlx::query_as(r#"SELECT id, invite_token FROM "Teams" WHERE UPPER(name) = UPPER($1)"#)
                .bind(&team_name)
                .fetch_optional(&mut *transaction)
                .await
                .map_err(|e| AppError::internal(e.to_string()))?;

        let (team_id, invite_token) = if let Some((tid, token)) = existing_team {
            (tid, token)
        } else {
            let invite_token = random_hex(16); // 32 hex chars
            let team_id: i32 = sqlx::query_scalar(
                r#"INSERT INTO "Teams"
                     (name, bio, avatar_hash, locked, deletion_pending, invite_token, captain_id)
                   VALUES ($1, NULL, NULL, FALSE, FALSE, $2, NULL)
                   RETURNING id"#,
            )
            .bind(&team_name)
            .bind(&invite_token)
            .fetch_one(&mut *transaction)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
            (team_id, invite_token)
        };

        // Check if team is already enrolled in this game
        let existing_part: Option<i32> = sqlx::query_scalar(
            r#"SELECT id FROM "Participations" WHERE game_id = $1 AND team_id = $2"#,
        )
        .bind(id)
        .bind(team_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

        let part_id = if let Some(pid) = existing_part {
            if model.division_id.is_some() {
                sqlx::query(r#"UPDATE "Participations" SET division_id = $1 WHERE id = $2"#)
                    .bind(model.division_id)
                    .bind(pid)
                    .execute(&mut *transaction)
                    .await
                    .map_err(|e| AppError::internal(e.to_string()))?;
            }
            pid
        } else {
            let part_token =
                crate::controllers::game::participation_token_from_key(&game.private_key, team_id)?;

            sqlx::query_scalar(
                r#"INSERT INTO "Participations"
                     (game_id, team_id, division_id, status, token, suspicion_score)
                   VALUES ($1, $2, $3, 0, $4, 0)
                   RETURNING id"#,
            )
            .bind(id)
            .bind(team_id)
            .bind(model.division_id)
            .bind(part_token)
            .fetch_one(&mut *transaction)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?
        };

        // If team already has members/captain, sync them into UserParticipations for this game
        sqlx::query(
            r#"INSERT INTO "UserParticipations" (user_id, game_id, team_id, participation_id)
               SELECT member.user_id, $1, $2, $3
                 FROM (
                     SELECT captain_id AS user_id FROM "Teams" WHERE id = $2 AND captain_id IS NOT NULL
                     UNION
                     SELECT user_id FROM "TeamMembers" WHERE team_id = $2
                 ) member
               ON CONFLICT (user_id, game_id) DO NOTHING"#,
        )
        .bind(id)
        .bind(team_id)
        .bind(part_id)
        .execute(&mut *transaction)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

        // Format invite code as {name}:{id}:{token}
        let invite_code = format!("{}:{}:{}", team_name, team_id, invite_token);

        results.push(SeededTeamResult {
            team_id,
            team_name,
            invite_code,
        });
    }

    transaction
        .commit()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    crate::services::audit::info(
        &st,
        "GameController",
        Some(user.name.clone()),
        None,
        format!("Seeded {} empty teams in game {}", results.len(), id),
    )
    .await;

    Ok(RequestResponse::ok(results))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_teams_model_deserialization() {
        let json = r#"{"teamNames":["Team 01","Team 02"],"divisionId":5}"#;
        let model: GameSeedTeamsModel = serde_json::from_str(json).unwrap();
        assert_eq!(model.team_names, vec!["Team 01", "Team 02"]);
        assert_eq!(model.division_id, Some(5));

        let json_default = r#"{"teamNames":["Team 01"]}"#;
        let model_default: GameSeedTeamsModel = serde_json::from_str(json_default).unwrap();
        assert_eq!(model_default.team_names, vec!["Team 01"]);
        assert_eq!(model_default.division_id, None);
    }

    #[test]
    fn seeded_team_result_serialization_is_camel_case() {
        let result = SeededTeamResult {
            team_id: 42,
            team_name: "Alpha".to_string(),
            invite_code: "Alpha:42:token123".to_string(),
        };
        let serialized = serde_json::to_string(&result).unwrap();
        assert!(serialized.contains(r#""teamId":42"#));
        assert!(serialized.contains(r#""teamName":"Alpha""#));
        assert!(serialized.contains(r#""inviteCode":"Alpha:42:token123""#));
    }

    #[test]
    fn team_validation_in_seed_teams() {
        assert!(crate::controllers::team::validate_team_profile(Some("Valid Name"), None).is_ok());
        let too_long = "a".repeat(crate::controllers::team::MAX_TEAM_NAME_CHARS + 1);
        assert!(crate::controllers::team::validate_team_profile(Some(&too_long), None).is_err());
    }
}
