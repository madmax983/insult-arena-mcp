//! Regression tests for Announcer.
//!
//! Verifies that the Announcer does not panic under edge case conditions,
//! such as 0 wins needed or malformed states.

use insult_arena_mcp::{Announcer, ArenaOutcome, DuelStateView, Duelist, Exchange, ExchangeResult};
use std::borrow::Cow;

#[test]
fn announcer_should_not_panic_on_zero_wins_needed() {
    let view = DuelStateView {
        phase: Cow::Borrowed("active"),
        next_to_act: None,
        pending_insult: None,
        challenger_score: 0,
        defender_score: 0,
        wins_needed: 0, // This is the trigger
        winner: None,
    };

    let outcome = ArenaOutcome::ExchangeProcessed {
        exchange: Exchange {
            attacker: Duelist::Challenger,
            result: ExchangeResult::Parried {
                insult: Cow::Borrowed("foo"),
                comeback: "bar".to_string(),
            },
            winner: Duelist::Defender,
        },
    };

    // This should NOT panic
    let result = Announcer::announce(&outcome, Some(&view));

    // We expect it to finish safely, even if the message is silly.
    assert!(!result.is_empty(), "Announcer returned empty string");
}
