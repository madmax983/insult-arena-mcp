//! Tests for stale duel cleanup.
//!
//! Verifies that duels can be reset after a timeout, preventing the server
//! from getting stuck in an abandoned state.

use insult_arena_mcp::Arena;
use insult_arena_mcp::arena::ArenaError;
use std::thread;
use std::time::Duration;

#[test]
#[allow(clippy::unwrap_used)]
fn test_stalled_duel_timeout_reset() {
    let mut arena = Arena::new();
    // Set a very short timeout
    arena.set_timeout(Duration::from_millis(10));

    arena.start_duel().unwrap();

    // Confirm that immediately starting a new duel fails
    let result = arena.start_duel();
    assert_eq!(result.err(), Some(ArenaError::DuelInProgress));

    // Wait for timeout
    thread::sleep(Duration::from_millis(20));

    // Confirm that starting a new duel NOW succeeds (reset)
    let result = arena.start_duel();
    assert!(result.is_ok(), "Should allow reset after timeout");
}
