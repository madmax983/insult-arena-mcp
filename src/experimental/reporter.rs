//! Narrative generation and match statistics.
//!
//! The Reporter module turns raw duel data into a story (`chronicle`) and extracts
//! interesting statistics (`analyze`) for post-game summaries.

use crate::{Duel, Duelist, ExchangeResult};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

/// Statistics derived from a duel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchStats {
    /// Total number of rounds (exchanges) played.
    pub total_rounds: usize,
    /// Percentage of successful parries (0.0 to 1.0).
    pub parry_rate: f64,
    /// Who won the match.
    pub winner: Option<Duelist>,
    /// The final score (Challenger, Defender).
    pub final_score: (u8, u8),
}

/// A reporter that generates narratives and stats for a duel.
///
/// # Hero's Journey
///
/// ```
/// use insult_arena_mcp::Duel;
/// use insult_arena_mcp::experimental::reporter::Reporter;
///
/// // 1. Play a duel
/// let mut duel = Duel::new();
/// duel.throw_insult("You fight like a dairy farmer!".to_string()).unwrap();
/// duel.respond("How appropriate. You fight like a cow!".to_string()).unwrap();
///
/// // 2. Generate the chronicle
/// let story = Reporter::chronicle(&duel);
/// println!("{}", story);
///
/// // 3. Analyze stats
/// let stats = Reporter::analyze(&duel);
/// assert!(stats.total_rounds > 0);
/// assert_eq!(stats.parry_rate, 1.0);
/// ```
pub struct Reporter;

impl Reporter {
    /// Analyzes a duel to produce statistics.
    ///
    /// Calculates total rounds, parry success rate, and captures the final result.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn analyze(duel: &Duel) -> MatchStats {
        let exchanges = duel.exchanges();
        let total_rounds = exchanges.len();

        let parries = exchanges.iter().filter(|e| e.result.is_parried()).count();
        let parry_rate = if total_rounds > 0 {
            parries as f64 / total_rounds as f64
        } else {
            0.0
        };

        let result = duel.result();
        let winner = result.as_ref().map(|r| r.winner);
        let final_score = duel.scores();

        MatchStats {
            total_rounds,
            parry_rate,
            winner,
            final_score,
        }
    }

    /// Generates a narrative chronicle of the duel.
    ///
    /// The chronicle is a Markdown-formatted story describing every exchange in the duel,
    /// complete with dramatic commentary.
    ///
    /// # Examples
    ///
    /// ```
    /// use insult_arena_mcp::Duel;
    /// use insult_arena_mcp::experimental::reporter::Reporter;
    ///
    /// let mut duel = Duel::new();
    /// duel.throw_insult("You fight like a dairy farmer!".to_string()).unwrap();
    /// duel.respond("wrong".to_string()).unwrap();
    ///
    /// let story = Reporter::chronicle(&duel);
    /// assert!(story.contains("The Ballad of the Insult Arena"));
    /// assert!(story.contains("Round 1"));
    /// ```
    #[must_use]
    pub fn chronicle(duel: &Duel) -> String {
        let mut story = String::new();

        story.push_str("# The Ballad of the Insult Arena\n\n");

        if duel.exchanges().is_empty() {
            story.push_str("The arena was silent. No insults were thrown.\n");
            return story;
        }

        for (i, exchange) in duel.exchanges().iter().enumerate() {
            let _ = writeln!(story, "## Round {}", i + 1);

            let attacker_name = format!("{}", exchange.attacker);
            let defender_name = format!("{}", exchange.attacker.opponent());

            match &exchange.result {
                ExchangeResult::Parried { insult, comeback } => {
                    let _ = writeln!(story, "**{attacker_name}** lunged: \"*{insult}*\"\n");
                    let _ = writeln!(
                        story,
                        "But **{defender_name}** was ready! \"*{comeback}*\"\n"
                    );
                    let _ = writeln!(
                        story,
                        "The crowd roared as **{defender_name}** gained the advantage!\n"
                    );
                }
                ExchangeResult::Failed {
                    insult,
                    attempt,
                    correct,
                } => {
                    let _ = writeln!(story, "**{attacker_name}** attacked: \"*{insult}*\"\n");
                    let _ = writeln!(story, "**{defender_name}** stammered: \"*{attempt}*\"\n");
                    let _ = writeln!(story, "(They should have said: \"*{correct}*\")\n");
                    let _ = writeln!(story, "**{attacker_name}** landed a solid hit!\n");
                }
            }
        }

        story.push_str("## Conclusion\n\n");
        if let Some(res) = duel.result() {
            let _ = writeln!(
                story,
                "**{}** stands victorious! Final Score: {}-{}",
                res.winner, res.challenger_score, res.defender_score
            );
        } else {
            story.push_str("The duel continues...\n");
        }

        story
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_empty_duel() {
        let duel = Duel::new();
        let stats = Reporter::analyze(&duel);

        assert_eq!(stats.total_rounds, 0);
        assert!((stats.parry_rate - 0.0).abs() < f64::EPSILON);
        assert!(stats.winner.is_none());
        assert_eq!(stats.final_score, (0, 0));
    }

    #[test]
    fn test_analyze_completed_duel() {
        let mut duel = Duel::with_wins_needed(1);
        duel.throw_insult("You fight like a dairy farmer!".to_string())
            .unwrap();
        duel.respond("wrong".to_string()).unwrap(); // Challenger wins

        let stats = Reporter::analyze(&duel);

        assert_eq!(stats.total_rounds, 1);
        assert!((stats.parry_rate - 0.0).abs() < f64::EPSILON);
        assert_eq!(stats.winner, Some(Duelist::Challenger));
        assert_eq!(stats.final_score, (1, 0));
    }

    #[test]
    fn test_chronicle_generation() {
        let mut duel = Duel::new();
        duel.throw_insult("You fight like a dairy farmer!".to_string())
            .unwrap();
        duel.respond("How appropriate. You fight like a cow!".to_string())
            .unwrap();

        let story = Reporter::chronicle(&duel);

        assert!(story.contains("The Ballad of the Insult Arena"));
        assert!(story.contains("Round 1"));
        assert!(story.contains("dairy farmer"));
        assert!(story.contains("like a cow"));
        assert!(story.contains("The crowd roared"));
    }
}
