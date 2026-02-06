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
    ///
    /// # Examples
    ///
    /// ```
    /// use insult_arena_mcp::{Announcer, ArenaOutcome, Duelist};
    ///
    /// // Duel started
    /// let msg = Announcer::announce(&ArenaOutcome::DuelStarted, None);
    /// assert!(msg.contains("En garde"));
    ///
    /// // Insult thrown
    /// let msg = Announcer::announce(
    ///     &ArenaOutcome::InsultThrown { insult: "You fight like a dairy farmer!".into() },
    ///     None
    /// );
    /// assert!(msg.contains("You bellow"));
    /// ```
    ///
    /// # Logic
    ///
    /// - Detects "Match Point" situations (score is `wins_needed - 1`) and adds tension text.
    /// - Distinguishes between winning an exchange vs winning the entire duel.
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
                let (score_display, is_finished, match_point_text) = view.map_or_else(
                    || ("(Score: ?-?)".to_string(), false, ""),
                    |view| {
                        let is_finished = view.phase == "finished";
                        let score_display =
                            format!("(Score: {}-{})", view.challenger_score, view.defender_score);

                        // GAME FEEL: Added Match Point notification to heighten tension near end-game (Ludwig)
                        let match_point_text = if !is_finished
                            && (view.challenger_score == view.wins_needed - 1
                                || view.defender_score == view.wins_needed - 1)
                        {
                            "\n\n🔥 MATCH POINT! 🔥 Next point wins!"
                        } else {
                            ""
                        };
                        (score_display, is_finished, match_point_text)
                    },
                );

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
