use super::{
    ad_submit_cost_units, bounded_result_label, is_plausible_flag, repeated_batch_result,
    require_active_victim, ActiveVictimFlag, ActiveVictimService, AdBatchSubmitResultModel,
    AdSubmitResultModel, ACCEPTED_ATTACK_SQL, ACTIVE_VICTIM_FLAG_SQL, AD_MAX_BATCH,
    AD_SUBMIT_BODY_BYTES,
};

#[test]
fn inactive_victim_preserves_wrong_status_and_planted_round() {
    assert_eq!(
        require_active_victim(None, Some(41)),
        Err(("wrong", Some(41)))
    );
}

#[test]
fn active_victim_continues_adjudication_with_exact_identifiers() {
    let expected = ActiveVictimService {
        id: 7,
        participation_id: 11,
        challenge_id: 13,
    };
    assert_eq!(
        require_active_victim(Some(expected), Some(41)).unwrap(),
        expected
    );
}

#[test]
fn victim_lookup_is_bounded_to_one_authoritative_active_service() {
    for predicate in [
        "WHERE flag = $1",
        "LIMIT 1",
        "WHERE service.id = planted.team_service_id",
        "AND service.game_id = $2",
        "AND victim.status = $3",
        "AND challenge.is_enabled = TRUE",
        "AND challenge.review_status = $4",
        "AND challenge.\"Type\" = $5",
    ] {
        assert!(
            ACTIVE_VICTIM_FLAG_SQL.contains(predicate),
            "missing active-victim predicate: {predicate}"
        );
    }
    assert!(ACTIVE_VICTIM_FLAG_SQL.contains("ORDER BY id DESC"));
    assert!(!ACTIVE_VICTIM_FLAG_SQL.contains("SELECT *"));
}

#[test]
fn planted_round_lookup_is_bounded_independently_of_round_history() {
    // `planted` is at most one row and the only AdRounds access follows its
    // primary-key id. Adding years of historical rounds therefore cannot
    // increase the rows materialized by a submit; in particular, never
    // restore the former game-wide `fetch_all` round map.
    for fragment in [
        r#"SELECT id, round_id, team_service_id"#,
        r#"ORDER BY id DESC"#,
        r#"LIMIT 1"#,
        r#"LEFT JOIN "AdRounds" planted_round"#,
        r#"ON planted_round.id = planted.round_id"#,
        r#"AND planted_round.game_id = $2"#,
        r#"planted_round.number AS planted_round_number"#,
    ] {
        assert!(
            ACTIVE_VICTIM_FLAG_SQL.contains(fragment),
            "missing bounded round-lookup invariant: {fragment}"
        );
    }
    assert_eq!(ACTIVE_VICTIM_FLAG_SQL.matches(r#""AdRounds""#).count(), 1);
    assert!(!ACTIVE_VICTIM_FLAG_SQL.contains(r#"SELECT id, number FROM "AdRounds" WHERE game_id"#));
}

#[test]
fn accepted_insert_returns_event_metadata_and_victim_scoped_first_blood() {
    for fragment in [
        r#"ON CONFLICT (attacker_participation_id, flag_id) DO NOTHING"#,
        r#"COALESCE(attacker_team.name, '') AS attacker_team"#,
        r#"victim_team.name AS victim_team"#,
        r#"challenge.title AS challenge_title"#,
        r#"prior_service.challenge_id = candidate.challenge_id"#,
        r#"prior_service.participation_id = candidate.victim_participation_id"#,
        r#"prior_attack.id <> inserted.id"#,
        r#"CASE WHEN candidate.broadcast_ok THEN NOT EXISTS"#,
    ] {
        assert!(
            ACCEPTED_ATTACK_SQL.contains(fragment),
            "missing accepted-insert invariant: {fragment}"
        );
    }
    assert!(ACCEPTED_ATTACK_SQL.contains("NOT game.hidden"));
    assert!(ACCEPTED_ATTACK_SQL.contains("game.freeze_time_utc IS NOT NULL"));
    assert_eq!(ACCEPTED_ATTACK_SQL.matches("INSERT INTO").count(), 1);
}

#[test]
fn inactive_service_keeps_the_flag_identity_but_cannot_be_attacked() {
    let flag = ActiveVictimFlag {
        flag_id: 17,
        planted_round_number: Some(19),
        service_id: None,
        participation_id: None,
        challenge_id: None,
    };
    assert_eq!(flag.active_service(), None);
}

#[test]
fn repeated_accepted_flag_is_reported_as_duplicate_without_double_counting() {
    assert_eq!(
        repeated_batch_result(("accepted", Some(41))),
        ("duplicate", Some(41))
    );
    assert_eq!(
        repeated_batch_result(("expired", Some(12))),
        ("expired", Some(12))
    );
}

#[test]
fn malformed_or_oversized_flags_never_reach_postgres() {
    assert!(is_plausible_flag("flag{ABCDEFGHIJKLMNOPQRSTUVWXYZabcd_-}"));
    for invalid in [
        "",
        "flag{short}",
        "FLAG{ABCDEFGHIJKLMNOPQRSTUVWXYZabcd_-}",
        "flag{ABCDEFGHIJKLMNOPQRSTUVWXYZabcd+/}",
        "flag{ABCDEFGHIJKLMNOPQRSTUVWXYZabcd_-}suffix",
    ] {
        assert!(!is_plausible_flag(invalid), "accepted {invalid:?}");
    }
    assert!(!is_plausible_flag(&"x".repeat(1024 * 1024)));
    assert_eq!(bounded_result_label(8, &"x".repeat(1024)), "#9");
    assert_eq!(ad_submit_cost_units(100, 3_800), 200);
    assert_eq!(ad_submit_cost_units(0, 3_800), 100);
}

#[test]
fn malformed_batch_results_never_reflect_attacker_sized_values() {
    let raw = "x".repeat(4_096);
    let results: Vec<_> = (0..AD_MAX_BATCH)
        .map(|index| AdSubmitResultModel {
            flag: bounded_result_label(index, &raw),
            status: "rejected".to_string(),
            flag_planted_at_round: None,
            message: Some("flag does not match the A&D grammar".to_string()),
        })
        .collect();
    let body = serde_json::to_vec(&AdBatchSubmitResultModel {
        accepted_count: 0,
        results,
    })
    .unwrap();
    assert!(body.len() < AD_SUBMIT_BODY_BYTES);
    assert!(!body
        .windows(raw.len())
        .any(|window| window == raw.as_bytes()));
}
