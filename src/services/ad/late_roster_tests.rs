use super::*;

use std::str::FromStr;

use sqlx::postgres::PgConnectOptions;
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires PostgreSQL via RSCTF_TEST_DATABASE_URL"]
async fn late_koth_admission_appends_once_and_backfills_zero_epochs() {
    let database_url = std::env::var("RSCTF_TEST_DATABASE_URL")
        .expect("RSCTF_TEST_DATABASE_URL must point to disposable PostgreSQL");
    let options = PgConnectOptions::from_str(&database_url).unwrap();
    let mut connection = PgConnection::connect_with(&options).await.unwrap();
    let schema = format!("rsctf_late_roster_{}", Uuid::new_v4().simple());
    sqlx::query(&format!(r#"CREATE SCHEMA "{schema}""#))
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::query(&format!(r#"SET search_path TO "{schema}""#))
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::raw_sql(
        r#"
        CREATE TABLE "KothOfficialConfigs" (
          game_id INTEGER PRIMARY KEY, roster_snapshot JSONB NOT NULL
        );
        CREATE TABLE "KothEpochRollups" (
          game_id INTEGER NOT NULL, epoch INTEGER NOT NULL,
          epoch_weight FLOAT8 NOT NULL, PRIMARY KEY (game_id, epoch)
        );
        CREATE TABLE "KothEpochTeamRollups" (
          game_id INTEGER NOT NULL, epoch INTEGER NOT NULL,
          participation_id INTEGER NOT NULL, points FLOAT8 NOT NULL,
          epoch_weight FLOAT8 NOT NULL, acquisition_rate FLOAT8 NOT NULL,
          control_rate FLOAT8 NOT NULL, sla_rate FLOAT8 NOT NULL,
          acquisition_windows BIGINT NOT NULL, controlled_ticks BIGINT NOT NULL,
          responsible_ticks BIGINT NOT NULL, healthy_responsible_ticks BIGINT NOT NULL,
          cumulative_points_numerator FLOAT8 NOT NULL,
          cumulative_epoch_weight FLOAT8 NOT NULL,
          cumulative_acquisition_numerator FLOAT8 NOT NULL,
          cumulative_control_numerator FLOAT8 NOT NULL,
          cumulative_sla_numerator FLOAT8 NOT NULL,
          cumulative_rate_weight FLOAT8 NOT NULL,
          cumulative_acquisition_windows BIGINT NOT NULL,
          cumulative_controlled_ticks BIGINT NOT NULL,
          cumulative_responsible_ticks BIGINT NOT NULL,
          cumulative_healthy_responsible_ticks BIGINT NOT NULL,
          PRIMARY KEY (game_id, epoch, participation_id)
        );
        CREATE TABLE "KothEpochHillRollups" (
          game_id INTEGER NOT NULL, epoch INTEGER NOT NULL,
          participation_id INTEGER NOT NULL, challenge_id INTEGER NOT NULL,
          service_weight FLOAT8 NOT NULL, evidence_fraction FLOAT8 NOT NULL,
          epoch_fraction FLOAT8 NOT NULL, local_points FLOAT8 NOT NULL,
          acquisition_rate FLOAT8 NOT NULL, control_rate FLOAT8 NOT NULL,
          sla_rate FLOAT8 NOT NULL, acquisition_windows BIGINT NOT NULL,
          controlled_ticks BIGINT NOT NULL, responsible_ticks BIGINT NOT NULL,
          healthy_responsible_ticks BIGINT NOT NULL,
          cumulative_points_numerator FLOAT8 NOT NULL,
          cumulative_score_weight FLOAT8 NOT NULL,
          cumulative_acquisition_numerator FLOAT8 NOT NULL,
          cumulative_control_numerator FLOAT8 NOT NULL,
          cumulative_sla_numerator FLOAT8 NOT NULL,
          cumulative_rate_weight FLOAT8 NOT NULL,
          cumulative_acquisition_windows BIGINT NOT NULL,
          cumulative_controlled_ticks BIGINT NOT NULL,
          cumulative_responsible_ticks BIGINT NOT NULL,
          cumulative_healthy_responsible_ticks BIGINT NOT NULL,
          PRIMARY KEY (game_id, epoch, participation_id, challenge_id)
        );
        INSERT INTO "KothOfficialConfigs" VALUES (7, '[11]'::jsonb);
        INSERT INTO "KothEpochRollups" VALUES (7,1,0.5), (7,2,0.5);
        INSERT INTO "KothEpochHillRollups" VALUES
          (7,1,11,9,1.0,1.0,0.5,80,0.8,0.8,1.0,1,1,1,1,
           40,0.5,0.4,0.4,0.5,0.5,1,1,1,1),
          (7,2,11,9,1.0,1.0,0.5,80,0.8,0.8,1.0,1,1,1,1,
           80,1.0,0.8,0.8,1.0,1.0,2,2,2,2);
        "#,
    )
    .execute(&mut connection)
    .await
    .unwrap();

    assert!(admit_late_koth_participation(&mut connection, 7, 12)
        .await
        .unwrap());
    assert!(!admit_late_koth_participation(&mut connection, 7, 12)
        .await
        .unwrap());
    assert_eq!(
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"SELECT roster_snapshot FROM "KothOfficialConfigs" WHERE game_id = 7"#,
        )
        .fetch_one(&mut connection)
        .await
        .unwrap(),
        serde_json::json!([11, 12])
    );
    let team_rows: Vec<(i32, f64, f64)> = sqlx::query_as(
        r#"SELECT epoch, points, cumulative_epoch_weight
             FROM "KothEpochTeamRollups"
            WHERE game_id = 7 AND participation_id = 12 ORDER BY epoch"#,
    )
    .fetch_all(&mut connection)
    .await
    .unwrap();
    assert_eq!(team_rows, vec![(1, 0.0, 0.5), (2, 0.0, 1.0)]);
    let hill_rows: Vec<(i32, f64, f64)> = sqlx::query_as(
        r#"SELECT epoch, local_points, cumulative_score_weight
             FROM "KothEpochHillRollups"
            WHERE game_id = 7 AND participation_id = 12 ORDER BY epoch"#,
    )
    .fetch_all(&mut connection)
    .await
    .unwrap();
    assert_eq!(hill_rows, vec![(1, 0.0, 0.5), (2, 0.0, 1.0)]);

    sqlx::query("SET search_path TO public")
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::query(&format!(r#"DROP SCHEMA "{schema}" CASCADE"#))
        .execute(&mut connection)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires PostgreSQL via RSCTF_TEST_DATABASE_URL"]
async fn late_ad_admission_seeds_placeholders_and_backfills_zero_epochs() {
    let database_url = std::env::var("RSCTF_TEST_DATABASE_URL")
        .expect("RSCTF_TEST_DATABASE_URL must point to disposable PostgreSQL");
    let options = PgConnectOptions::from_str(&database_url).unwrap();
    let mut connection = PgConnection::connect_with(&options).await.unwrap();
    let schema = format!("rsctf_late_ad_roster_{}", Uuid::new_v4().simple());
    sqlx::query(&format!(r#"CREATE SCHEMA "{schema}""#))
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::query(&format!(r#"SET search_path TO "{schema}""#))
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::raw_sql(
        r#"
        CREATE TABLE "Games" (
          id INTEGER PRIMARY KEY, ad_scoring_start_round INTEGER
        );
        CREATE TABLE "Participations" (
          id INTEGER PRIMARY KEY, game_id INTEGER NOT NULL, status SMALLINT NOT NULL
        );
        CREATE TABLE "GameChallenges" (
          id INTEGER PRIMARY KEY, game_id INTEGER NOT NULL, is_enabled BOOLEAN NOT NULL,
          review_status SMALLINT NOT NULL, "Type" SMALLINT NOT NULL,
          ad_scoring_weight FLOAT8 NOT NULL
        );
        CREATE TABLE "AdTeamServices" (
          id SERIAL PRIMARY KEY, game_id INTEGER NOT NULL,
          participation_id INTEGER NOT NULL, challenge_id INTEGER NOT NULL,
          host TEXT NOT NULL, port INTEGER NOT NULL, status SMALLINT NOT NULL,
          container_id TEXT, last_reset_at TIMESTAMPTZ,
          UNIQUE (participation_id, challenge_id)
        );
        CREATE TABLE "AdEpochRollups" (
          game_id INTEGER NOT NULL, epoch INTEGER NOT NULL,
          epoch_weight FLOAT8 NOT NULL, PRIMARY KEY (game_id, epoch)
        );
        CREATE TABLE "AdEpochTeamRollups" (
          game_id INTEGER NOT NULL, epoch INTEGER NOT NULL,
          participation_id INTEGER NOT NULL, points FLOAT8 NOT NULL,
          epoch_weight FLOAT8 NOT NULL, cumulative_points_numerator FLOAT8 NOT NULL,
          cumulative_epoch_weight FLOAT8 NOT NULL,
          cumulative_offense_numerator FLOAT8 NOT NULL,
          cumulative_defense_numerator FLOAT8 NOT NULL,
          cumulative_sla_numerator FLOAT8 NOT NULL,
          cumulative_rate_weight FLOAT8 NOT NULL,
          PRIMARY KEY (game_id, epoch, participation_id)
        );
        CREATE TABLE "AdEpochServiceRollups" (
          game_id INTEGER NOT NULL, epoch INTEGER NOT NULL,
          participation_id INTEGER NOT NULL, challenge_id INTEGER NOT NULL,
          service_weight FLOAT8 NOT NULL, opportunity_count BIGINT NOT NULL,
          capture_count BIGINT NOT NULL, rarity_sum FLOAT8 NOT NULL,
          defense_opportunity_count BIGINT NOT NULL,
          protected_opportunity_count BIGINT NOT NULL,
          sla_credit_sum FLOAT8 NOT NULL, sla_tick_count BIGINT NOT NULL,
          closing_sla_status SMALLINT, closing_sla_credit FLOAT8,
          local_points FLOAT8 NOT NULL, offense_rate FLOAT8 NOT NULL,
          defense_rate FLOAT8 NOT NULL, sla_rate FLOAT8 NOT NULL,
          cumulative_points_numerator FLOAT8 NOT NULL,
          cumulative_epoch_weight FLOAT8 NOT NULL,
          cumulative_offense_numerator FLOAT8 NOT NULL,
          cumulative_defense_numerator FLOAT8 NOT NULL,
          cumulative_sla_numerator FLOAT8 NOT NULL,
          cumulative_capture_count BIGINT NOT NULL,
          PRIMARY KEY (game_id, epoch, participation_id, challenge_id)
        );
        INSERT INTO "Games" VALUES (42, 1);
        INSERT INTO "Participations" VALUES (101, 42, 1);
        INSERT INTO "GameChallenges" VALUES (501, 42, TRUE, 0, 4, 1.0);
        INSERT INTO "AdEpochRollups" VALUES (42, 1, 1.0), (42, 2, 1.0);
        "#,
    )
    .execute(&mut connection)
    .await
    .unwrap();

    assert!(admit_late_ad_participation(&mut connection, 42, 101)
        .await
        .unwrap());

    let team_rollups: Vec<(i32, f64, f64, f64)> = sqlx::query_as(
        r#"SELECT epoch, points, cumulative_epoch_weight, cumulative_points_numerator
             FROM "AdEpochTeamRollups"
            WHERE game_id = 42 AND participation_id = 101 ORDER BY epoch"#,
    )
    .fetch_all(&mut connection)
    .await
    .unwrap();
    assert_eq!(team_rollups, vec![(1, 0.0, 1.0, 0.0), (2, 0.0, 2.0, 0.0)]);

    let service_rollups: Vec<(i32, i32, f64, f64)> = sqlx::query_as(
        r#"SELECT epoch, challenge_id, local_points, cumulative_epoch_weight
             FROM "AdEpochServiceRollups"
            WHERE game_id = 42 AND participation_id = 101 ORDER BY epoch"#,
    )
    .fetch_all(&mut connection)
    .await
    .unwrap();
    assert_eq!(
        service_rollups,
        vec![(1, 501, 0.0, 1.0), (2, 501, 0.0, 2.0)]
    );

    sqlx::query("SET search_path TO public")
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::query(&format!(r#"DROP SCHEMA "{schema}" CASCADE"#))
        .execute(&mut connection)
        .await
        .unwrap();
}
