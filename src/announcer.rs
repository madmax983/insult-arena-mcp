use crate::arena::{ArenaOutcome, DuelStateView};
use crate::duel::{Duelist, ExchangeResult};
use std::fmt::Write;

/// Helper struct to format scores without allocation.
struct ScoreDisplay<'a>(Option<&'a DuelStateView>);

impl std::fmt::Display for ScoreDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(view) => write!(
                f,
                "(Score: {}-{})",
                view.challenger_score, view.defender_score
            ),
            None => write!(f, "(Score: ?-?)"),
        }
    }
}

/// Handles the generation of flavor text and commentary for game events.
pub struct Announcer;

impl Announcer {
    /// Generates a human-readable announcement for an arena outcome.
    ///
    /// # Arguments
    ///
    /// * `outcome` - The event that just occurred.
    /// * `view` - The current state of the duel (required for score-related announcements).
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
                let (is_finished, match_point_text) = view.map_or((false, ""), |view| {
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
                    (is_finished, match_point_text)
                });

                let score_display = ScoreDisplay(view);

                if exchange.result.is_parried() {
                    if is_finished {
                        let _ = write!(
                            f,
                            "🏆 VICTORY! {} has won the duel! {}",
                            exchange.winner, score_display
                        );
                    } else {
                        let _ = write!(
                            f,
                            "⚔️ TOUCHÉ! A sharp wit! {} wins the exchange and attacks next! {}{}",
                            exchange.winner, score_display, match_point_text
                        );
                    }
                } else {
                    let expected =
                        if let ExchangeResult::Failed { ref correct, .. } = exchange.result {
                            correct.as_str()
                        } else {
                            ""
                        };

                    if is_finished {
                        let _ = write!(
                            f,
                            "💥 OOF! That didn't land! {} wins the duel! {}\n\nExpected comeback: \"{}\"",
                            exchange.winner, score_display, expected
                        );
                    } else {
                        let _ = write!(
                            f,
                            "💥 OOF! That didn't land! {} wins the exchange and attacks again! {}{}\n\nExpected comeback: \"{}\"",
                            exchange.winner, score_display, match_point_text, expected
                        );
                    }
                }
            }
        }
        f
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duel::Exchange;

    fn mock_view(c_score: u8, d_score: u8, phase: &str) -> DuelStateView {
        DuelStateView {
            phase: phase.to_string(),
            next_to_act: None,
            pending_insult: None,
            challenger_score: c_score,
            defender_score: d_score,
            wins_needed: 3,
            winner: None,
        }
    }

    #[test]
    fn announce_duel_started() {
        let msg = Announcer::announce(&ArenaOutcome::DuelStarted, None);
        assert!(msg.contains("En garde"));
    }

    #[test]
    fn announce_insult_thrown() {
        let msg = Announcer::announce(
            &ArenaOutcome::InsultThrown {
                insult: "Foo".into(),
            },
            None,
        );
        assert!(msg.contains("You bellow: \"Foo\""));
    }

    #[test]
    fn announce_exchange_parried_mid_game() {
        let view = mock_view(1, 1, "awaiting_insult");
        let exchange = Exchange {
            attacker: Duelist::Challenger,
            result: ExchangeResult::Parried {
                insult: "i".into(),
                comeback: "c".into(),
            },
            winner: Duelist::Defender,
        };

        let msg = Announcer::announce(&ArenaOutcome::ExchangeProcessed { exchange }, Some(&view));

        assert!(msg.contains("TOUCHÉ"));
        assert!(msg.contains("Defender wins the exchange"));
        assert!(msg.contains("(Score: 1-1)"));
        assert!(!msg.contains("MATCH POINT"));
    }

    #[test]
    fn announce_exchange_failed_mid_game_match_point() {
        // Challenger 2-1 Defender. Challenger needs 1 more win.
        let view = mock_view(2, 1, "awaiting_insult");
        let exchange = Exchange {
            attacker: Duelist::Defender,
            result: ExchangeResult::Failed {
                insult: "i".into(),
                attempt: "wrong".into(),
                correct: "right".into(),
            },
            winner: Duelist::Defender, // Attacker (Defender) wins because they threw insult and opponent failed?
                                       // Wait, if Defender threw insult and Challenger failed to respond, Defender wins exchange.
        };

        let msg = Announcer::announce(&ArenaOutcome::ExchangeProcessed { exchange }, Some(&view));

        assert!(msg.contains("OOF"));
        assert!(msg.contains("Defender wins the exchange"));
        assert!(msg.contains("(Score: 2-1)"));
        assert!(msg.contains("MATCH POINT"));
        assert!(msg.contains("Expected comeback: \"right\""));
    }

    #[test]
    fn announce_exchange_victory() {
        let view = mock_view(3, 1, "finished");
        let exchange = Exchange {
            attacker: Duelist::Challenger,
            result: ExchangeResult::Parried {
                insult: "i".into(),
                comeback: "c".into(),
            },
            winner: Duelist::Defender, // Hypothethical
        };
        // Wait, if Challenger has 3, Challenger won.
        // But exchange winner is passed in outcome.

        let msg = Announcer::announce(&ArenaOutcome::ExchangeProcessed { exchange }, Some(&view));
        assert!(msg.contains("VICTORY"));
        assert!(msg.contains("(Score: 3-1)"));
    }

    #[test]
    fn announce_exchange_no_view_fallback() {
        let exchange = Exchange {
            attacker: Duelist::Challenger,
            result: ExchangeResult::Parried {
                insult: "i".into(),
                comeback: "c".into(),
            },
            winner: Duelist::Defender,
        };
        let msg = Announcer::announce(&ArenaOutcome::ExchangeProcessed { exchange }, None);
        assert!(msg.contains("(Score: ?-?)"));
    }
}
