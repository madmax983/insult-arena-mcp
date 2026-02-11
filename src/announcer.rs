//! Text generation for game events.
//!
//! The [`Announcer`] is a stateless formatter that converts internal game events
//! (like [`ArenaOutcome`]) into human-readable strings. It handles:
//!
//! - Flavor text (e.g., "En garde!", "OOF!").
//! - Score reporting.
//! - Match point notifications.
//! - Context-aware messages (e.g., winning exchange vs winning duel).

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
        let mut f = String::new();

        match outcome {
            ArenaOutcome::DuelStarted => {
                let _ = write!(
                    f,
                    "⚔️ En garde! A new duel begins. Challenger, throw the first insult! 🏴‍☠️"
                );
            }
            ArenaOutcome::RoleRegistered { role } => match role {
                Duelist::Challenger => {
                    let _ = write!(
                        f,
                        "🏴‍☠️ You are the CHALLENGER! Sharpen your tongue and throw the first insult!"
                    );
                }
                Duelist::Defender => {
                    let _ = write!(
                        f,
                        "🛡️ You are the DEFENDER! Brace yourself for insults and retort with a comeback!"
                    );
                }
            },
            ArenaOutcome::InsultThrown { insult } => {
                let _ = write!(f, "🗣️ You bellow: \"{insult}\" ... awaiting comeback!");
            }
            ArenaOutcome::ExchangeProcessed { exchange } => {
                let (scores, is_finished, match_point_text) = view.map_or_else(
                    || (None, false, ""),
                    |view| {
                        let is_finished = view.phase == "finished";

                        // GAME FEEL: Added Match Point notification to heighten tension near end-game (Ludwig)
                        let match_point_text = if !is_finished
                            && (view.challenger_score == view.wins_needed - 1
                                || view.defender_score == view.wins_needed - 1)
                        {
                            "\n\n🔥 MATCH POINT! 🔥 Next point wins!"
                        } else {
                            ""
                        };
                        (
                            Some((view.challenger_score, view.defender_score)),
                            is_finished,
                            match_point_text,
                        )
                    },
                );

                // ⚡ Bolt Optimization:
                // Removed intermediate `score_display` string allocation (via `format!`).
                // Now writes scores directly to the result buffer.
                if exchange.result.is_parried() {
                    if is_finished {
                        let _ = write!(f, "🏆 VICTORY! {} has won the duel! ", exchange.winner);
                    } else {
                        let _ = write!(
                            f,
                            "⚔️ TOUCHÉ! A sharp wit! {} wins the exchange and attacks next! ",
                            exchange.winner
                        );
                    }
                } else {
                    let _ = write!(f, "💥 OOF! That didn't land! ");
                    if is_finished {
                        let _ = write!(f, "{} wins the duel! ", exchange.winner);
                    } else {
                        let _ = write!(
                            f,
                            "{} wins the exchange and attacks again! ",
                            exchange.winner
                        );
                    }
                }

                // Append score directly
                if let Some((challenger, defender)) = scores {
                    let _ = write!(f, "(Score: {challenger}-{defender})");
                } else {
                    let _ = write!(f, "(Score: ?-?)");
                }

                // Append match point text
                let _ = write!(f, "{match_point_text}");

                // Append expected comeback if failed
                if let ExchangeResult::Failed { ref correct, .. } = exchange.result {
                    let _ = write!(f, "\n\nExpected comeback: \"{correct}\"");
                }
            }
        }
        f
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
            phase: if finished { "finished" } else { "active" }.to_string(),
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
                            insult: "i".to_string(),
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
                            insult: "i".to_string(),
                            attempt: "bad".to_string(),
                            correct: "correct".to_string(),
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
                            insult: "i".to_string(),
                            attempt: "bad".to_string(),
                            correct: "correct".to_string(),
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
                            insult: "i".to_string(),
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
                            insult: "i".to_string(),
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
                            insult: "i".to_string(),
                            attempt: "bad".to_string(),
                            correct: "correct".to_string(),
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
                            insult: "i".to_string(),
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
}
