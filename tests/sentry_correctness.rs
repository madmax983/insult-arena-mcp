#![cfg(feature = "nova")]
#![allow(clippy::unwrap_used)]

use insult_arena_mcp::experimental::audience::{Audience, HISTORY_LIMIT, Reaction};
use insult_arena_mcp::{Duel, Duelist, Exchange, ExchangeResult};

#[test]
fn test_audience_unicode_normalization_dedupes() {
    let mut audience = Audience::new();

    // 1. Play "¡Hola!" (Spanish)
    let exchange1 = Exchange {
        attacker: Duelist::Challenger,
        result: ExchangeResult::Parried {
            insult: "¡Hola!".into(),
            comeback: "¡Adiós!".to_string(),
        },
        winner: Duelist::Defender,
    };
    let reaction1 = audience.react(&exchange1, None);
    assert!(
        matches!(reaction1, Reaction::Cheer(_)),
        "First use should cheer"
    );

    // 2. Play "¡HOLA!" (Spanish Uppercase)
    // Should be normalized to "hola" (if '¡' is stripped and 'H' -> 'h').
    // Audience::normalize keeps alphanumeric. '¡' is Punctuation/Symbol.
    // 'H' is alphanumeric. 'O' is alphanumeric.
    let exchange2 = Exchange {
        attacker: Duelist::Challenger,
        result: ExchangeResult::Parried {
            insult: "¡HOLA!".into(),
            comeback: "¡Adiós!".to_string(),
        },
        winner: Duelist::Defender,
    };
    let reaction2 = audience.react(&exchange2, None);

    assert!(
        matches!(reaction2, Reaction::Boo(_)),
        "Second use (different case) should be booed as repetition"
    );
}

#[test]
fn test_audience_unicode_different_languages_unique() {
    let mut audience = Audience::new();

    // 1. "Hello" (English)
    let exchange1 = Exchange {
        attacker: Duelist::Challenger,
        result: ExchangeResult::Parried {
            insult: "Hello".into(),
            comeback: "Hi".to_string(),
        },
        winner: Duelist::Defender,
    };
    audience.react(&exchange1, None);

    // 2. "Bonjour" (French)
    let exchange2 = Exchange {
        attacker: Duelist::Challenger,
        result: ExchangeResult::Parried {
            insult: "Bonjour".into(),
            comeback: "Salut".to_string(),
        },
        winner: Duelist::Defender,
    };
    let reaction = audience.react(&exchange2, None);

    assert!(
        matches!(reaction, Reaction::Cheer(_)),
        "Different language should be unique"
    );
}

#[test]
fn test_duel_state_invariants_next_to_act() {
    // Verify that for all non-finished states, next_to_act is Some.
    // This ensures the notification message "It's unknown's turn!" is unreachable logic.

    let mut duel = Duel::new();

    // Initial state: AwaitingInsult (Challenger)
    let view = insult_arena_mcp::DuelStateView::from(&duel);
    assert_eq!(view.phase, "awaiting_insult");
    assert_eq!(view.next_to_act.as_deref(), Some("Challenger"));

    // Throw insult -> AwaitingComeback (Defender)
    duel.throw_insult("You fight like a dairy farmer!".to_string())
        .unwrap();
    let view = insult_arena_mcp::DuelStateView::from(&duel);
    assert_eq!(view.phase, "awaiting_comeback");
    assert_eq!(view.next_to_act.as_deref(), Some("Defender"));

    // Respond (Parry) -> AwaitingInsult (Defender)
    duel.respond("How appropriate. You fight like a cow!".to_string())
        .unwrap();
    let view = insult_arena_mcp::DuelStateView::from(&duel);
    assert_eq!(view.phase, "awaiting_insult");
    assert_eq!(view.next_to_act.as_deref(), Some("Defender"));

    // Respond (Fail) would transition to AwaitingInsult (Attacker wins exchange).
    // Let's force a win to check finished state.
    // Need 3 wins. Defender has 1.
    // Defender attacks. Challenger fails. (Defender 2)
    duel.throw_insult("You have the manners of a beggar.".to_string())
        .unwrap();
    duel.respond("wrong".to_string()).unwrap();

    // Defender attacks. Challenger fails. (Defender 3 - Win)
    duel.throw_insult("You have the manners of a beggar.".to_string())
        .unwrap();
    duel.respond("wrong".to_string()).unwrap();

    let view = insult_arena_mcp::DuelStateView::from(&duel);
    assert_eq!(view.phase, "finished");
    assert!(
        view.next_to_act.is_none(),
        "Finished duel should have no next actor"
    );
    assert_eq!(view.winner.as_deref(), Some("Defender"));
}

#[test]
fn test_audience_history_limit_strict() {
    let mut audience = Audience::new();

    // Fill history to limit
    for i in 0..HISTORY_LIMIT {
        let exchange = Exchange {
            attacker: Duelist::Challenger,
            result: ExchangeResult::Parried {
                insult: format!("Unique insult {i}").into(),
                comeback: "c".into(),
            },
            winner: Duelist::Defender,
        };
        let reaction = audience.react(&exchange, None);
        assert!(
            matches!(reaction, Reaction::Cheer(_)),
            "Should cheer unique insult {i}"
        );
    }

    // Add one more (limit + 1). Should evict "Unique insult 0".
    let exchange_overflow = Exchange {
        attacker: Duelist::Challenger,
        result: ExchangeResult::Parried {
            insult: "Overflow insult".into(),
            comeback: "c".into(),
        },
        winner: Duelist::Defender,
    };
    audience.react(&exchange_overflow, None);

    // Verify "Unique insult 0" is forgotten (can use it again without booing)
    let exchange_reuse = Exchange {
        attacker: Duelist::Challenger,
        result: ExchangeResult::Parried {
            insult: "Unique insult 0".into(), // Was first
            comeback: "c".into(),
        },
        winner: Duelist::Defender,
    };
    let reaction = audience.react(&exchange_reuse, None);

    assert!(
        matches!(reaction, Reaction::Cheer(_)),
        "Should cheer reused insult after eviction"
    );
}

#[test]
fn test_search_insults_ignores_punctuation() {
    use insult_arena_mcp::InsultBank;
    let bank = InsultBank::new();

    // "dairy-farmer" should match "You fight like a dairy farmer!"
    // Current logic fails this because it compares raw query "dairy-farmer" against "You fight like a dairy farmer!"
    // Expected logic: normalize both (strip punctuation) and compare.

    let results = bank.search_insults("dairy-farmer");
    assert!(
        !results.is_empty(),
        "Should find 'dairy farmer' even with hyphen in query"
    );

    let results = bank.search_insults("dairy farmer");
    assert!(
        !results.is_empty(),
        "Should find 'dairy farmer' with space"
    );
}
