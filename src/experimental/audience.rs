use std::collections::VecDeque;

use crate::{Exchange, ExchangeResult};
use serde::{Deserialize, Serialize};

/// Maximum number of insults to remember in history.
///
/// This limit prevents the history from growing indefinitely, which could lead to
/// memory exhaustion (`DoS`). Once the limit is reached, the oldest insults are forgotten.
pub const HISTORY_LIMIT: usize = 50;

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
///     Reaction::Cheer(msg) => println!("👏 Crowd Cheers: {}", msg),
///     Reaction::Laugh(msg) => println!("😂 Crowd Laughs: {}", msg),
///     Reaction::Boo(msg) => println!("👎 Crowd Boos: {}", msg),
///     Reaction::Silence => println!("😐 Crowd is silent."),
/// }
/// assert!(audience.hype > 50);
/// ```
#[derive(Debug, Clone)]
pub struct Audience {
    /// Hype level from 0 to 100. Starts at 50.
    pub hype: i32,
    /// History of insults used to detect repetition.
    ///
    /// Limited to [`HISTORY_LIMIT`] items to prevent `DoS` via memory exhaustion.
    history: VecDeque<String>,
}

impl Default for Audience {
    fn default() -> Self {
        Self::new()
    }
}

impl Audience {
    /// Creates a new audience with initial hype of 50.
    #[must_use]
    pub fn new() -> Self {
        Self {
            hype: 50,
            history: VecDeque::with_capacity(HISTORY_LIMIT),
        }
    }

    /// Helper to normalize strings for repetition detection.
    /// Retains only alphanumeric characters and converts to lowercase.
    ///
    /// # Security
    ///
    /// Truncates the input to [`crate::arena::MAX_INPUT_LENGTH`] to prevent excessive allocation.
    fn normalize(s: &str) -> String {
        // 🛡️ SENTRY: Truncate input to prevent DoS via massive allocation.
        // We use bytes length check first for speed, then char slicing if needed.
        let s = if s.len() > crate::arena::MAX_INPUT_LENGTH {
            // Find char boundary to avoid panic
            let mut len = crate::arena::MAX_INPUT_LENGTH;
            while !s.is_char_boundary(len) {
                len -= 1;
            }
            &s[..len]
        } else {
            s
        };

        s.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect()
    }

    /// Reacts to an exchange, updating hype and returning a reaction.
    ///
    /// # Logic
    ///
    /// 1.  **Normalization**: The insult is normalized (lowercase, alphanumeric only) to detect repetition.
    /// 2.  **Repetition Check**: If the insult is in the recent history ([`HISTORY_LIMIT`]), the audience BOOs (-20 hype).
    /// 3.  **Result Check**:
    ///     -   **Parry**: The audience CHEERS (+10 hype).
    ///     -   **Fail**: The audience LAUGHS (-10 hype).
    ///
    /// # Examples
    ///
    /// ```
    /// use insult_arena_mcp::experimental::audience::{Audience, Reaction};
    /// use insult_arena_mcp::{Exchange, Duelist, ExchangeResult};
    ///
    /// let mut audience = Audience::new();
    /// let exchange = Exchange {
    ///     attacker: Duelist::Challenger,
    ///     winner: Duelist::Defender,
    ///     result: ExchangeResult::Parried {
    ///         insult: "You fight like a dairy farmer!".into(),
    ///         comeback: "How appropriate. You fight like a cow!".into()
    ///     }
    /// };
    ///
    /// let reaction = audience.react(&exchange);
    /// assert!(matches!(reaction, Reaction::Cheer(_)));
    /// ```
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

        // Add to history and enforce limit (FIFO)
        if self.history.len() >= HISTORY_LIMIT {
            self.history.pop_front();
        }
        self.history.push_back(normalized);

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

    #[test]
    fn test_history_bounded() {
        let mut audience = Audience::new();

        // 1. Fill history with unique insults (0 to HISTORY_LIMIT inclusive -> HISTORY_LIMIT + 1 items)
        for i in 0..=HISTORY_LIMIT {
            let exchange = Exchange {
                attacker: Duelist::Challenger,
                result: ExchangeResult::Parried {
                    insult: format!("insult {i}").into(),
                    comeback: "comeback".into(),
                },
                winner: Duelist::Defender,
            };
            audience.react(&exchange);
        }

        // 2. Reuse the oldest insult ("insult 0")
        // If history is bounded (size HISTORY_LIMIT), "insult 0" should have been evicted.
        let exchange = Exchange {
            attacker: Duelist::Challenger,
            result: ExchangeResult::Parried {
                insult: "insult 0".into(),
                comeback: "comeback".into(),
            },
            winner: Duelist::Defender,
        };
        let reaction = audience.react(&exchange);

        // EXPECTATION: Should NOT Boo if history is bounded.
        // CURRENT BUG: Returns Boo because history is unbounded.
        assert!(
            !matches!(reaction, Reaction::Boo(_)),
            "History should be bounded! Old insult caused Boo."
        );
    }
}
