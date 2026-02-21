//! Text generation for game events.
//!
//! # The Storyteller of the Arena
//!
//! The [`Announcer`] is responsible for converting the raw data of the game state ([`ArenaOutcome`])
//! into the colorful commentary that players see. It is the "voice" of the game.
//!
//! ## Design
//!
//! - **Stateless**: The Announcer holds no data. It is a pure function over the input state.
//! - **Context-Aware**: It adapts messages based on game context (e.g., announcing "Match Point!" when scores are critical).
//! - **Flavorful**: It uses emojis and dramatic punctuation to create an engaging atmosphere.
//!
//! ## Usage
//!
//! The Announcer is typically used by the [`crate::InsultServer`] to generate the `message` field
//! of the JSON-RPC response.
//!
//! ```
//! use insult_arena_mcp::{Announcer, ArenaOutcome};
//!
//! let outcome = ArenaOutcome::DuelStarted;
//! let message = Announcer::announce(&outcome, None);
//! println!("{}", message); // "⚔️ En garde! ..."
//! ```

use crate::arena::ArenaOutcome;
use crate::duel::{DuelStateView, Duelist, ExchangeResult};
use std::fmt::Write;

/// The announcer responsible for generating commentary.
///
/// This struct is purely a namespace for static methods and holds no state.
pub struct Announcer;

impl Announcer {
    /// Generates a human-readable announcement for an arena outcome.
    ///
    /// # Arguments
    ///
    /// * `outcome` - The event that just occurred ([`ArenaOutcome`]).
    /// * `view` - The current state of the duel (required for score-related announcements).
    ///
    /// # Logic
    ///
    /// The announcer adapts the message based on the game state:
    ///
    /// 1. **Match Point**: If a player is one point away from winning (`wins_needed - 1`),
    ///    it appends a "MATCH POINT!" warning to heighten tension.
    /// 2. **Victory**: Checks if the duel is finished to distinguish between "winning an exchange"
    ///    and "winning the duel".
    /// 3. **Roles**: Provides role-specific welcome messages when players register.
    ///
    /// # Examples
    ///
    /// ```
    /// use insult_arena_mcp::{Announcer, ArenaOutcome, Duelist};
    ///
    /// // 1. Duel Start
    /// let msg = Announcer::announce(&ArenaOutcome::DuelStarted, None);
    /// assert!(msg.contains("En garde"));
    ///
    /// // 2. Player Registration
    /// let msg = Announcer::announce(
    ///     &ArenaOutcome::RoleRegistered { role: Duelist::Challenger },
    ///     None
    /// );
    /// assert!(msg.contains("You are the CHALLENGER"));
    ///
    /// // 3. Insult Thrown
    /// let msg = Announcer::announce(
    ///     &ArenaOutcome::InsultThrown { insult: "You fight like a dairy farmer!".into() },
    ///     None
    /// );
    /// assert!(msg.contains("You bellow"));
    /// ```
    #[must_use]
    pub fn announce(outcome: &ArenaOutcome, view: Option<&DuelStateView>) -> String {
        // ⚡ Bolt Optimization: Pre-allocate buffer to avoid reallocations.
        // Average message size is ~100-200 bytes.
        let mut f = String::with_capacity(256);

        match outcome {
            ArenaOutcome::DuelStarted => Self::announce_started(&mut f),
            ArenaOutcome::RoleRegistered { role } => Self::announce_registered(&mut f, *role),
            ArenaOutcome::InsultThrown { insult } => Self::announce_insult(&mut f, insult),
            ArenaOutcome::ExchangeProcessed { exchange } => {
                Self::format_exchange(&mut f, exchange, view);
            }
        }
        f
    }

    fn announce_started(f: &mut String) {
        f.push_str("⚔️ En garde! A new duel begins. Challenger, throw the first insult! 🏴‍☠️");
    }

    fn announce_registered(f: &mut String, role: Duelist) {
        match role {
            Duelist::Challenger => {
                f.push_str(
                    "🏴‍☠️ You are the CHALLENGER! Sharpen your tongue and throw the first insult!",
                );
            }
            Duelist::Defender => {
                f.push_str(
                    "🛡️ You are the DEFENDER! Brace yourself for insults and retort with a comeback!",
                );
            }
        }
    }

    #[allow(clippy::unwrap_used)]
    fn announce_insult(f: &mut String, insult: &str) {
        write!(f, "🗣️ You bellow: \"{insult}\" ... awaiting comeback!").unwrap();
    }

    fn format_exchange(
        f: &mut String,
        exchange: &crate::duel::Exchange,
        view: Option<&DuelStateView>,
    ) {
        let is_finished = view.is_some_and(|v| v.phase == "finished");

        Self::append_outcome_description(f, exchange, is_finished);

        let scores = view.map(|v| (v.challenger_score, v.defender_score));
        Self::format_score_text(f, scores);

        if let Some(v) = view {
            f.push_str(Self::get_match_point_text(v));
        }

        // Append expected comeback if failed
        if let ExchangeResult::Failed { ref correct, .. } = exchange.result {
            #[allow(clippy::unwrap_used)]
            write!(f, "\n\nExpected comeback: \"{correct}\"").unwrap();
        }
    }

    /// Helper to format the result of the exchange (Victory, Touché, or Oof).
    #[allow(clippy::unwrap_used)]
    fn append_outcome_description(
        f: &mut String,
        exchange: &crate::duel::Exchange,
        is_finished: bool,
    ) {
        let winner = exchange.winner;
        let is_parried = exchange.result.is_parried();

        // 1. Describe the exchange
        if is_parried {
            if !is_finished {
                write!(
                    f,
                    "⚔️ TOUCHÉ! A sharp wit! {winner} wins the exchange and attacks next! "
                )
                .unwrap();
            }
        } else {
            f.push_str("💥 OOF! That didn't land! ");
        }

        // 2. Describe the final outcome (if finished) or if failed but continues (logic from original)
        if is_finished {
            if is_parried {
                write!(f, "🏆 VICTORY! {winner} has won the duel! ").unwrap();
            } else {
                write!(f, "{winner} wins the duel! ").unwrap();
            }
        } else if !is_parried {
            // Failed and not finished
            write!(f, "{winner} wins the exchange and attacks again! ").unwrap();
        }
    }

    /// Helper to format the score string.
    #[allow(clippy::unwrap_used)]
    fn format_score_text(f: &mut String, scores: Option<(u8, u8)>) {
        if let Some((challenger, defender)) = scores {
            write!(f, "(Score: {challenger}-{defender})").unwrap();
        } else {
            f.push_str("(Score: ?-?)");
        }
    }

    /// Helper to determine if we should display "MATCH POINT!".
    fn get_match_point_text(view: &DuelStateView) -> &'static str {
        // GAME FEEL: Added Match Point notification to heighten tension near end-game (Ludwig)
        if view.phase != "finished"
            && view.wins_needed > 0
            && (view.challenger_score == view.wins_needed - 1
                || view.defender_score == view.wins_needed - 1)
        {
            "\n\n🔥 MATCH POINT! 🔥 Next point wins!"
        } else {
            ""
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duel::{Exchange, ExchangeResult};

    struct TestCase {
        name: &'static str,
        outcome: ArenaOutcome,
        view: Option<DuelStateView>,
        expected_contains: Vec<&'static str>,
    }

    fn make_view(c_score: u8, d_score: u8, finished: bool) -> DuelStateView {
        DuelStateView {
            phase: std::borrow::Cow::Borrowed(if finished { "finished" } else { "active" }),
            next_to_act: None,
            pending_insult: None,
            challenger_score: c_score,
            defender_score: d_score,
            wins_needed: 3,
            winner: None,
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_announcer_table_driven() {
        let cases = vec![
            TestCase {
                name: "Duel Started",
                outcome: ArenaOutcome::DuelStarted,
                view: None,
                expected_contains: vec!["En garde", "Challenger, throw the first insult"],
            },
            TestCase {
                name: "Register Challenger",
                outcome: ArenaOutcome::RoleRegistered {
                    role: Duelist::Challenger,
                },
                view: None,
                expected_contains: vec!["You are the CHALLENGER"],
            },
            TestCase {
                name: "Register Defender",
                outcome: ArenaOutcome::RoleRegistered {
                    role: Duelist::Defender,
                },
                view: None,
                expected_contains: vec!["You are the DEFENDER"],
            },
            TestCase {
                name: "Insult Thrown",
                outcome: ArenaOutcome::InsultThrown {
                    insult: "You fight like a dairy farmer!".to_string(),
                },
                view: None,
                expected_contains: vec!["You bellow", "You fight like a dairy farmer!"],
            },
            TestCase {
                name: "Exchange Parried (Standard)",
                outcome: ArenaOutcome::ExchangeProcessed {
                    exchange: Exchange {
                        attacker: Duelist::Challenger,
                        result: ExchangeResult::Parried {
                            insult: "i".into(),
                            comeback: "c".to_string(),
                        },
                        winner: Duelist::Defender,
                    },
                },
                view: Some(make_view(0, 1, false)),
                expected_contains: vec!["TOUCHÉ", "Defender wins the exchange", "(Score: 0-1)"],
            },
            TestCase {
                name: "Exchange Failed (Standard)",
                outcome: ArenaOutcome::ExchangeProcessed {
                    exchange: Exchange {
                        attacker: Duelist::Challenger,
                        result: ExchangeResult::Failed {
                            insult: "i".into(),
                            attempt: "bad".to_string(),
                            correct: "correct".into(),
                        },
                        winner: Duelist::Challenger,
                    },
                },
                view: Some(make_view(1, 0, false)),
                expected_contains: vec![
                    "OOF",
                    "Challenger wins the exchange",
                    "Expected comeback",
                    "\"correct\"",
                    "(Score: 1-0)",
                ],
            },
            TestCase {
                name: "Match Point Challenger",
                outcome: ArenaOutcome::ExchangeProcessed {
                    exchange: Exchange {
                        attacker: Duelist::Defender,
                        result: ExchangeResult::Failed {
                            insult: "i".into(),
                            attempt: "bad".to_string(),
                            correct: "correct".into(),
                        },
                        winner: Duelist::Challenger,
                    },
                },
                view: Some(make_view(2, 0, false)), // 2 wins, need 3. Match point!
                expected_contains: vec!["MATCH POINT!", "Next point wins!"],
            },
            TestCase {
                name: "Match Point Defender",
                outcome: ArenaOutcome::ExchangeProcessed {
                    exchange: Exchange {
                        attacker: Duelist::Challenger,
                        result: ExchangeResult::Parried {
                            insult: "i".into(),
                            comeback: "c".to_string(),
                        },
                        winner: Duelist::Defender,
                    },
                },
                view: Some(make_view(1, 2, false)), // Defender has 2. Match point!
                expected_contains: vec!["MATCH POINT!", "Next point wins!"],
            },
            TestCase {
                name: "Victory (Parried)",
                outcome: ArenaOutcome::ExchangeProcessed {
                    exchange: Exchange {
                        attacker: Duelist::Challenger,
                        result: ExchangeResult::Parried {
                            insult: "i".into(),
                            comeback: "c".to_string(),
                        },
                        winner: Duelist::Defender,
                    },
                },
                view: Some(make_view(0, 3, true)),
                expected_contains: vec!["VICTORY!", "Defender has won the duel", "(Score: 0-3)"],
            },
            TestCase {
                name: "Victory (Failed)",
                outcome: ArenaOutcome::ExchangeProcessed {
                    exchange: Exchange {
                        attacker: Duelist::Defender,
                        result: ExchangeResult::Failed {
                            insult: "i".into(),
                            attempt: "bad".to_string(),
                            correct: "correct".into(),
                        },
                        winner: Duelist::Challenger,
                    },
                },
                view: Some(make_view(3, 0, true)),
                expected_contains: vec!["OOF", "Challenger wins the duel", "(Score: 3-0)"],
            },
            TestCase {
                name: "View is None (Fallback)",
                outcome: ArenaOutcome::ExchangeProcessed {
                    exchange: Exchange {
                        attacker: Duelist::Challenger,
                        result: ExchangeResult::Parried {
                            insult: "i".into(),
                            comeback: "c".to_string(),
                        },
                        winner: Duelist::Defender,
                    },
                },
                view: None,
                expected_contains: vec!["(Score: ?-?)"],
            },
        ];

        for case in cases {
            let result = Announcer::announce(&case.outcome, case.view.as_ref());
            for substring in case.expected_contains {
                assert!(
                    result.contains(substring),
                    "Test '{}' failed: Output '{}' did not contain '{}'",
                    case.name,
                    result,
                    substring
                );
            }
        }
    }

    #[test]
    fn test_announcer_match_point_logic() {
        // Case: 3-2 Victory.
        // Winner has 3. Loser has 2 (which is wins_needed - 1).
        // Since is_finished is true, "MATCH POINT!" should NOT appear.
        let view = make_view(3, 2, true);
        let outcome = ArenaOutcome::ExchangeProcessed {
            exchange: Exchange {
                attacker: Duelist::Defender,
                result: ExchangeResult::Failed {
                    insult: "i".into(),
                    attempt: "bad".to_string(),
                    correct: "correct".into(),
                },
                winner: Duelist::Challenger,
            },
        };

        let msg = Announcer::announce(&outcome, Some(&view));

        assert!(msg.contains("VICTORY") || msg.contains("wins the duel"));
        assert!(msg.contains("(Score: 3-2)"));
        assert!(
            !msg.contains("MATCH POINT"),
            "Match point warning should not appear when duel is finished, even if loser is close."
        );
    }
}
