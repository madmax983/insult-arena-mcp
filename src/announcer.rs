use crate::arena::{ArenaOutcome, DuelStateView};
use crate::duel::{Duelist, ExchangeResult};
use std::fmt::Write;

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
