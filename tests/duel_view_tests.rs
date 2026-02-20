#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use insult_arena_mcp::{Duel, DuelStateView};
use serde_json::Value;

#[test]
fn test_view_awaiting_insult_start() {
    let duel = Duel::new();
    let view = DuelStateView::from(&duel);

    assert_eq!(view.phase, "awaiting_insult");
    assert_eq!(view.next_to_act.as_deref(), Some("Challenger"));
    assert_eq!(view.pending_insult, None);
    assert_eq!(view.challenger_score, 0);
    assert_eq!(view.defender_score, 0);
    assert_eq!(view.winner, None);
}

#[test]
fn test_view_awaiting_comeback() {
    let mut duel = Duel::new();
    let insult = "You fight like a dairy farmer!".to_string();
    duel.throw_insult(insult.clone()).unwrap();

    let view = DuelStateView::from(&duel);

    assert_eq!(view.phase, "awaiting_comeback");
    // Challenger threw insult, so Defender is next to act
    assert_eq!(view.next_to_act.as_deref(), Some("Defender"));
    assert_eq!(view.pending_insult.as_deref(), Some(insult.as_str()));
    assert_eq!(view.challenger_score, 0);
    assert_eq!(view.defender_score, 0);
    assert_eq!(view.winner, None);
}

#[test]
fn test_view_awaiting_insult_after_parry() {
    let mut duel = Duel::new();
    duel.throw_insult("You fight like a dairy farmer!".to_string())
        .unwrap();
    duel.respond("How appropriate. You fight like a cow!".to_string())
        .unwrap();

    let view = DuelStateView::from(&duel);

    assert_eq!(view.phase, "awaiting_insult");
    // Defender won, so Defender attacks next
    assert_eq!(view.next_to_act.as_deref(), Some("Defender"));
    assert_eq!(view.pending_insult, None);
    // Defender score increased
    assert_eq!(view.defender_score, 1);
    assert_eq!(view.challenger_score, 0);
    assert_eq!(view.winner, None);
}

#[test]
fn test_view_awaiting_insult_after_fail() {
    let mut duel = Duel::new();
    duel.throw_insult("You fight like a dairy farmer!".to_string())
        .unwrap();
    duel.respond("wrong".to_string()).unwrap();

    let view = DuelStateView::from(&duel);

    assert_eq!(view.phase, "awaiting_insult");
    // Challenger won (failed comeback), so Challenger attacks again
    assert_eq!(view.next_to_act.as_deref(), Some("Challenger"));
    assert_eq!(view.pending_insult, None);
    // Challenger score increased
    assert_eq!(view.challenger_score, 1);
    assert_eq!(view.defender_score, 0);
    assert_eq!(view.winner, None);
}

#[test]
fn test_view_finished() {
    let mut duel = Duel::with_wins_needed(1);
    duel.throw_insult("You fight like a dairy farmer!".to_string())
        .unwrap();
    duel.respond("wrong".to_string()).unwrap();

    // Challenger wins exchange -> 1-0 -> Game Over

    let view = DuelStateView::from(&duel);

    assert_eq!(view.phase, "finished");
    assert_eq!(view.next_to_act, None); // No one acts next
    assert_eq!(view.pending_insult, None);
    assert_eq!(view.challenger_score, 1);
    assert_eq!(view.defender_score, 0);
    assert_eq!(view.winner.as_deref(), Some("Challenger"));
}

#[test]
fn test_serialization_round_trip() {
    let mut duel = Duel::new();
    duel.throw_insult("You fight like a dairy farmer!".to_string())
        .unwrap();

    let view = DuelStateView::from(&duel);

    // Serialize
    let json = serde_json::to_string(&view).expect("Failed to serialize");

    // Check specific fields presence in JSON
    let parsed: Value = serde_json::from_str(&json).expect("Failed to parse JSON");
    assert_eq!(parsed["phase"], "awaiting_comeback");
    assert_eq!(parsed["next_to_act"], "Defender");
    assert_eq!(parsed["pending_insult"], "You fight like a dairy farmer!");

    // Check missing fields (Option::None should be omitted or null depending on serde setting)
    // In DuelStateView: #[serde(skip_serializing_if = "Option::is_none")]
    // So "winner" should be MISSING.
    assert!(parsed.get("winner").is_none());

    // Deserialize back
    let view_back: DuelStateView = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(view_back.phase, view.phase);
    assert_eq!(view_back.next_to_act, view.next_to_act);
    assert_eq!(view_back.pending_insult, view.pending_insult);
    assert_eq!(view_back.winner, view.winner);
}

#[test]
fn test_serialization_finished_omits_next_act() {
    let mut duel = Duel::with_wins_needed(1);
    duel.throw_insult("You fight like a dairy farmer!".to_string())
        .unwrap();
    duel.respond("wrong".to_string()).unwrap();

    let view = DuelStateView::from(&duel);
    let json = serde_json::to_string(&view).unwrap();
    let parsed: Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["phase"], "finished");
    assert!(parsed.get("next_to_act").is_none()); // Should be skipped
    assert_eq!(parsed["winner"], "Challenger");
}

#[test]
fn test_serialization_pending_insult_skipped() {
    let mut duel = Duel::new();
    let view = DuelStateView::from(&duel);

    // Initial state: pending_insult is None
    assert!(view.pending_insult.is_none());

    let json = serde_json::to_string(&view).unwrap();
    let parsed: Value = serde_json::from_str(&json).unwrap();

    // Verify "pending_insult" key is ABSENT
    assert!(
        parsed.get("pending_insult").is_none(),
        "pending_insult should be skipped when null"
    );

    // Throw insult
    duel.throw_insult("You fight like a dairy farmer!".to_string())
        .unwrap();
    let view = DuelStateView::from(&duel);
    assert!(view.pending_insult.is_some());

    let json = serde_json::to_string(&view).unwrap();
    let parsed: Value = serde_json::from_str(&json).unwrap();

    // Verify "pending_insult" key is PRESENT
    assert!(
        parsed.get("pending_insult").is_some(),
        "pending_insult should be present when set"
    );
    assert_eq!(parsed["pending_insult"], "You fight like a dairy farmer!");
}
