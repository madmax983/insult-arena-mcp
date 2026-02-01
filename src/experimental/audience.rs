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

/// A virtual audience that tracks the "hype" of the duel.
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

    /// Reacts to an exchange, updating hype and returning a reaction.
    pub fn react(&mut self, exchange: &Exchange) -> Reaction {
        let insult = match &exchange.result {
            ExchangeResult::Parried { insult, .. } | ExchangeResult::Failed { insult, .. } => {
                insult
            }
        };

        // Check for repetition
        if self.history.contains(insult) {
            self.hype = (self.hype - 20).max(0);
            return Reaction::Boo("Get new material!".into());
        }

        self.history.push(insult.clone());

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
