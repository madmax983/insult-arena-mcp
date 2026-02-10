use insult_arena_mcp::{Arena, ArenaOutcome, Duelist};
use insult_arena_mcp::arena::ArenaError;

#[test]
#[allow(clippy::unwrap_used)]
fn test_unrestricted_game_reset_dos() {
    let mut arena = Arena::new();

    // 1. Start a game
    let (outcome, _) = arena.start_duel().unwrap();
    assert!(matches!(outcome, ArenaOutcome::DuelStarted));

    // 2. Register players and make progress
    arena.register_challenger("alice".to_string()).unwrap();
    arena.register_defender("bob".to_string()).unwrap();

    arena.throw_insult("alice", "You fight like a dairy farmer!".to_string()).unwrap();

    // Verify state is "mid-game"
    let (view, _) = arena.get_duel_state(None).unwrap();
    assert_eq!(view.phase, "awaiting_comeback");

    // 3. ATTACK: Malicious actor calls start_duel() again
    // This simulates an arbitrary tool call from any client
    let result = arena.start_duel();

    // 4. Verify the defense: Game state is PROTECTED!
    assert!(result.is_err(), "start_duel should fail when a game is in progress");
    assert_eq!(result.unwrap_err(), ArenaError::DuelInProgress);

    // 5. Verify state is STILL "mid-game"
    let (view, _) = arena.get_duel_state(None).unwrap();
    assert_eq!(view.phase, "awaiting_comeback");

    // 6. Verify players are NOT kicked
    let (_, _role) = arena.get_duel_state(Some("alice")).unwrap();
    assert!(matches!(arena.get_role_for_session("alice"), Some(Duelist::Challenger)));
}
