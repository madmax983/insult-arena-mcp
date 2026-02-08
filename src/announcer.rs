//! # Announcer
//!
//! Provides flavor text and commentary for the insult sword fighting arena.
//!
//! This module decouples the game logic (in `Arena` and `Duel`) from the presentation
//! layer. It takes raw game events (`ArenaOutcome`) and state (`DuelStateView`) and
//! transforms them into engaging, pirate-themed messages for the players.

use crate::arena::{ArenaOutcome, DuelStateView};
use crate::duel::{Duelist, ExchangeResult};
use std::fmt::Write;

/// Handles the generation of flavor text and commentary for game events.
///
/// The `Announcer` is a stateless utility that formats game outcomes into
/// human-readable strings. It handles:
/// - Duel start announcements
/// - Role registration confirmations
/// - Insult delivery
/// - Exchange results (hit/miss)
/// - Victory/Defeat messages
///
/// # Examples
///
/// ```
/// use insult_arena_mcp::{Announcer, ArenaOutcome};
///
/// // Announce the start of a duel
/// let message = Announcer::announce(&ArenaOutcome::DuelStarted, None);
/// assert!(message.contains("En garde"));
/// ```
pub struct Announcer;

impl Announcer {
    /// Generates a human-readable announcement for an arena outcome.
    ///
    /// Takes the raw event and optional game state to produce a context-aware message.
    /// For example, if a player wins an exchange, the message will include the current score.
    ///
    /// # Arguments
    ///
    /// * `outcome` - The event that just occurred (e.g., `InsultThrown`, `ExchangeProcessed`).
    /// * `view` - The current state of the duel. This is optional but recommended for
    ///   outcomes that involve scoring (like `ExchangeProcessed`), as it provides
    ///   context like the current score and match point status.
    ///
    /// # Returns
    ///
    /// A `String` containing the formatted announcement, including emojis and flavor text.
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
