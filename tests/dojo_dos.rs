//! `DoS` protection tests for the Dojo.
//!
//! Verifies that the Dojo mode enforces input limits to prevent memory exhaustion attacks.

#![cfg(feature = "nova")]

use insult_arena_mcp::arena::ArenaError;
use insult_arena_mcp::experimental::dojo::Dojo;

#[test]
#[allow(clippy::expect_used)]
fn dojo_unbounded_allocation_dos() {
    let mut dojo = Dojo::new(0.5);

    // Create a massive string (e.g. 10MB)
    let massive_input = "a".repeat(10 * 1024 * 1024);

    let result = dojo.turn(&massive_input);

    // Expect failure due to length limit
    assert!(result.is_err(), "Dojo should reject massive input");

    // Check error type
    let err = result.expect_err("Should be an error");
    match err {
        ArenaError::InputTooLong(_) => (),
        _ => panic!("Expected InputTooLong error, got: {err:?}"),
    }
}
