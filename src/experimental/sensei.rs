use crate::InsultBank;
use rand::Rng;

/// The Sensei (AI) opponent in the Dojo.
#[derive(Debug, Clone)]
pub struct Sensei {
    /// Skill level (0.0 to 1.0).
    skill: f64,
}

impl Sensei {
    /// Creates a new Sensei with the given skill level.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(skill: f64) -> Self {
        // Clamp skill between 0.0 and 1.0
        Self {
            skill: skill.clamp(0.0, 1.0),
        }
    }

    /// Decides on an insult to throw.
    #[must_use]
    pub fn attack(&self, bank: &InsultBank) -> String {
        // Sensei always picks a valid insult
        bank.random_insult().insult.to_string()
    }

    /// Decides on a comeback response.
    #[must_use]
    pub fn defend(&self, bank: &InsultBank, pending_insult: &str) -> String {
        let should_succeed = rand::thread_rng().gen_bool(self.skill);

        if should_succeed {
            // Find correct comeback
            bank.find_comeback(pending_insult)
                .unwrap_or("...")
                .to_string()
        } else {
            // Fail intentionally
            "I am rubber, you are glue!".to_string()
        }
    }
}
