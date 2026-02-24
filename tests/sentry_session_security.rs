//! Security tests for session and role enforcement.
//!
//! Verifies that only registered sessions can act, and only in their assigned turn.
//! prevents session hijacking and out-of-turn actions.

#![allow(clippy::unwrap_used)]

use insult_arena_mcp::Arena;
use insult_arena_mcp::arena::{ArenaError, PlayerInput, SessionId};

#[test]
fn unregistered_session_cannot_act() {
    let mut arena = Arena::new();
    arena.start_duel().unwrap();

    let session_id = SessionId::try_from("unregistered_session".to_string()).unwrap();
    let insult = PlayerInput::try_from("You fight like a dairy farmer!".to_string()).unwrap();

    // Attempt to throw insult without registering as Challenger
    let result = arena.throw_insult(&session_id, insult);

    // This should fail because the session is not registered as Challenger.
    // Currently (bug), this might succeed if validate_turn logic is loose.
    // We assert it FAILS (Red Phase).
    assert!(
        matches!(result, Err(ArenaError::NotYourTurn(_))),
        "Expected NotYourTurn error, but got: {result:?}",
    );
}

#[test]
fn registered_session_can_act() {
    let mut arena = Arena::new();
    arena.start_duel().unwrap();

    let session_id = SessionId::try_from("registered_session".to_string()).unwrap();
    arena.register_challenger(session_id.clone()).unwrap();

    let insult = PlayerInput::try_from("You fight like a dairy farmer!".to_string()).unwrap();

    // Should succeed
    let result = arena.throw_insult(&session_id, insult);
    assert!(
        result.is_ok(),
        "Expected success, but got error: {:?}",
        result.err()
    );
}
