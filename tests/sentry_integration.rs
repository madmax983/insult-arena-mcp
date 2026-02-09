use insult_arena_mcp::{Announcer, Arena, ArenaOutcome};

#[test]
#[allow(clippy::unwrap_used)]
fn full_duel_integration_with_match_point() {
    let mut arena = Arena::new();
    let (outcome, view) = arena.start_duel();
    assert_eq!(view.phase, "awaiting_insult");
    assert!(matches!(outcome, ArenaOutcome::DuelStarted));

    // Register sessions
    arena.register_challenger("Alice".to_string()).unwrap();
    arena.register_defender("Bob".to_string()).unwrap();

    // Duel flow:
    // 1. Alice throws insult.
    // 2. Bob responds correctly. (Bob score: 1)
    // 3. Bob throws insult.
    // 4. Alice fails. (Bob score: 2) -> MATCH POINT
    // 5. Bob throws insult.
    // 6. Alice fails. (Bob score: 3) -> WIN

    // 1. Alice throws
    let insult = "You fight like a dairy farmer!";
    let (outcome, view) = arena.throw_insult("Alice", insult).unwrap();
    assert_eq!(view.phase, "awaiting_comeback");
    assert!(matches!(outcome, ArenaOutcome::InsultThrown { .. }));

    // 2. Bob responds (Correct)
    let comeback = "How appropriate. You fight like a cow!";
    let (outcome, view) = arena.respond("Bob", comeback).unwrap();
    assert_eq!(view.defender_score, 1);
    assert_eq!(view.phase, "awaiting_insult");
    assert_eq!(view.next_to_act, Some("Defender".to_string()));

    // Verify announcement
    let msg = Announcer::announce(&outcome, Some(&view));
    assert!(msg.contains("TOUCHÉ"));
    assert!(msg.contains("Score: 0-1"));

    // 3. Bob throws
    let insult = "You have the manners of a beggar.";
    let (_, _view) = arena.throw_insult("Bob", insult).unwrap();

    // 4. Alice fails
    let wrong_comeback = "I am rubber, you are glue.";
    let (outcome, view) = arena.respond("Alice", wrong_comeback).unwrap();
    assert_eq!(view.defender_score, 2);
    assert_eq!(view.phase, "awaiting_insult"); // Bob won exchange, so Bob attacks
    assert_eq!(view.next_to_act, Some("Defender".to_string()));

    // MATCH POINT CHECK
    // Score is 0-2. Wins needed is 3. Bob needs 1 more.
    // Announcer should say "MATCH POINT!"
    let msg = Announcer::announce(&outcome, Some(&view));
    assert!(
        msg.contains("MATCH POINT"),
        "Announcement should contain match point warning"
    );

    // 5. Bob throws (for the win)
    let insult = "Nobody's ever drawn blood from me and nobody ever will!";
    let (_, _view) = arena.throw_insult("Bob", insult).unwrap();

    // 6. Alice fails
    let wrong_comeback = "You run fast?";
    let (outcome, view) = arena.respond("Alice", wrong_comeback).unwrap();
    assert_eq!(view.defender_score, 3);
    assert_eq!(view.phase, "finished");
    assert_eq!(view.winner, Some("Defender".to_string()));

    // VICTORY CHECK
    let msg = Announcer::announce(&outcome, Some(&view));
    assert!(msg.contains("wins the duel"), "Should announce winner");
    assert!(msg.contains("Defender"));
    assert!(
        msg.contains("OOF"),
        "Should indicate failure caused the win"
    );
}
