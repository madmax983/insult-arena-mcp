use crate::{Exchange, ExchangeResult};
use serde::{Deserialize, Serialize};

/// Represents the crowd's reaction to an exchange.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reaction {
    /// The crowd cheers for a sick burn or great comeback.
    Cheer(String),
    /// The crowd laughs at a failure.
    Laugh(String),
    /// The crowd boos at repetition or boring play.
    Boo(String),
    /// The crowd is silent / neutral.
    Silence,
}

/// Maximum number of past insults to remember for repetition checking.
///
/// Prevents unbounded memory growth (DoS) in long-running duels.
const MAX_HISTORY_SIZE: usize = 50;

/// A virtual audience that tracks the "hype" of the duel.
///
/// # Rules of the Crowd
///
/// The audience has a short attention span and loves novelty.
///
/// - **Hype**: Starts at 50. Max 100, Min 0.
/// - **Parry**: **+10 Hype** (They love a witty comeback).
/// - **Fail**: **-10 Hype** (They cringe at failure).
/// - **Repetition**: **-20 Hype** (They boo unoriginal insults).
///
/// # Hero's Journey
///
/// ```
/// use insult_arena_mcp::experimental::audience::{Audience, Reaction};
/// use insult_arena_mcp::{Exchange, Duelist, ExchangeResult};
///
/// // 1. Create an audience
/// let mut audience = Audience::new();
///
/// // 2. Create an exchange
/// let exchange = Exchange {
///     attacker: Duelist::Challenger,
///     winner: Duelist::Defender,
///     result: ExchangeResult::Parried {
///         insult: "You fight like a dairy farmer!".into(),
///         comeback: "How appropriate. You fight like a cow!".into()
///     }
/// };
///
/// // 3. React!
/// let reaction = audience.react(&exchange);
/// match reaction {
///     Reaction::Cheer(msg) => println!("👏 {}", msg),
///     _ => println!("😐"),
/// }
/// assert!(audience.hype > 50);
/// ```
#[derive(Debug, Clone)]
pub struct Audience {
    /// Hype level from 0 to 100. Starts at 50.
    pub hype: i32,
    /// History of insults used to detect repetition.
    history: Vec<String>,
}

impl Default for Audience {
    fn default() -> Self {
        Self::new()
    }
}

impl Audience {
    /// Creates a new audience with initial hype of 50.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            hype: 50,
            history: Vec::new(),
        }
    }

    /// Helper to normalize strings for repetition detection.
    /// Retains only alphanumeric characters and converts to lowercase.
    fn normalize(s: &str) -> String {
        s.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect()
    }

    /// Reacts to an exchange, updating hype and returning a reaction.
    pub fn react(&mut self, exchange: &Exchange) -> Reaction {
        let insult = match &exchange.result {
            ExchangeResult::Parried { insult, .. } | ExchangeResult::Failed { insult, .. } => {
                insult
            }
        };

        let normalized = Self::normalize(insult);

        // Check for repetition
        if self.history.contains(&normalized) {
            self.hype = (self.hype - 20).max(0);
            return Reaction::Boo("Get new material!".into());
        }

        self.history.push(normalized);
        // 🛡️ HARDENING: Prevent memory exhaustion by capping history size.
        if self.history.len() > MAX_HISTORY_SIZE {
            self.history.remove(0);
        }

        match &exchange.result {
            ExchangeResult::Parried { .. } => {
                self.hype = (self.hype + 10).min(100);
                Reaction::Cheer("OOH! SICK BURN!".into())
            }
            ExchangeResult::Failed { .. } => {
                self.hype = (self.hype - 10).max(0);
                Reaction::Laugh("LOL! FAIL!".into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Duelist, Exchange, ExchangeResult};

    #[test]
    fn test_history_limit() {
        let mut audience = Audience::new();
        // Fill history beyond capacity
        for i in 0..60 {
            let exchange = Exchange {
                attacker: Duelist::Challenger,
                result: ExchangeResult::Parried {
                    insult: format!("insult {}", i).into(),
                    comeback: "comeback".into(),
                },
                winner: Duelist::Defender,
            };
            audience.react(&exchange);
        }

        assert_eq!(
            audience.history.len(),
            MAX_HISTORY_SIZE,
            "History size should be capped"
        );
        // Verify the oldest are removed.
        // History stores normalized insults. "insult 0" -> "insult0".
        // The last added was "insult 59" -> "insult59".
        // So "insult 0" to "insult 9" should be gone.
        // "insult 10" should be present.
        assert!(
            !audience.history.contains(&"insult0".to_string()),
            "Oldest entry should be removed"
        );
        assert!(
            audience.history.contains(&"insult59".to_string()),
            "Newest entry should be present"
        );
    }

    #[test]
    fn test_parry_increases_hype() {
        let mut audience = Audience::new();
        let initial_hype = audience.hype;

        let exchange = Exchange {
            attacker: Duelist::Challenger,
            result: ExchangeResult::Parried {
                insult: "You fight like a dairy farmer!".into(),
                comeback: "How appropriate. You fight like a cow!".into(),
            },
            winner: Duelist::Defender,
        };

        let reaction = audience.react(&exchange);

        assert!(
            audience.hype > initial_hype,
            "Hype should increase on parry"
        );
        assert!(
            matches!(reaction, Reaction::Cheer(_)),
            "Audience should cheer"
        );
    }

    #[test]
    fn test_fail_decreases_hype() {
        let mut audience = Audience::new();
        let initial_hype = audience.hype;

        let exchange = Exchange {
            attacker: Duelist::Challenger,
            result: ExchangeResult::Failed {
                insult: "You fight like a dairy farmer!".into(),
                attempt: "No u".into(),
                correct: "How appropriate. You fight like a cow!".into(),
            },
            winner: Duelist::Challenger,
        };

        let reaction = audience.react(&exchange);

        assert!(audience.hype < initial_hype, "Hype should decrease on fail");
        assert!(
            matches!(reaction, Reaction::Laugh(_)),
            "Audience should laugh"
        );
    }

    #[test]
    fn test_repetition_boos() {
        let mut audience = Audience::new();

        // First time is fine
        let exchange = Exchange {
            attacker: Duelist::Challenger,
            result: ExchangeResult::Parried {
                insult: "You fight like a dairy farmer!".into(),
                comeback: "How appropriate. You fight like a cow!".into(),
            },
            winner: Duelist::Defender,
        };
        audience.react(&exchange);

        // Second time should boo
        let initial_hype = audience.hype;
        let reaction = audience.react(&exchange);

        assert!(
            audience.hype < initial_hype,
            "Hype should decrease significantly on repetition"
        );
        assert!(
            matches!(reaction, Reaction::Boo(_)),
            "Audience should boo repetition"
        );
    }
}

#[cfg(test)]
mod sentry_repro_tests {
    use super::*;
    use crate::{Duelist, Exchange, ExchangeResult};

    #[test]
    fn repetition_evasion_should_fail() {
        let mut audience = Audience::new();

        // 1. First usage: "You fight like a dairy farmer!"
        let exchange1 = Exchange {
            attacker: Duelist::Challenger,
            result: ExchangeResult::Parried {
                insult: "You fight like a dairy farmer!".into(),
                comeback: "How appropriate. You fight like a cow!".into(),
            },
            winner: Duelist::Defender,
        };
        audience.react(&exchange1);

        // 2. Second usage: "YOU FIGHT LIKE A DAIRY FARMER!" (Different case)
        let exchange2 = Exchange {
            attacker: Duelist::Challenger,
            result: ExchangeResult::Parried {
                insult: "YOU FIGHT LIKE A DAIRY FARMER!".into(),
                comeback: "How appropriate. You fight like a cow!".into(),
            },
            winner: Duelist::Defender,
        };
        let reaction = audience.react(&exchange2);

        // EXPECTATION: Audience should Boo because it's the same insult.
        // CURRENT BUG: Audience will likely Cheer because "You fight..." != "YOU FIGHT..."
        assert!(
            matches!(reaction, Reaction::Boo(_)),
            "Audience should boo repeated insult even with different casing"
        );
    }
}
