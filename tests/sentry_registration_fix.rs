#![allow(clippy::unwrap_used)]
use insult_arena_mcp::{
    Arena,
    arena::{ArenaError, SessionId},
};

#[test]
fn test_registration_without_duel_fails() {
    let mut arena = Arena::new();

    let alice = SessionId::try_from("Alice".to_string()).unwrap();

    // 1. Register Challenger BEFORE start_duel
    // SENTRY FIX: This must now return Err(NoDuel)
    let result = arena.register_challenger(alice);

    assert!(
        result.is_err(),
        "Registration must fail without active duel"
    );
    assert!(matches!(result.unwrap_err(), ArenaError::NoDuel));
}
