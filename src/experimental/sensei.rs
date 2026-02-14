use crate::InsultBank;
use rand::Rng;

/// The Sensei (AI) opponent in the Dojo.
///
/// # Hero's Journey
///
/// ```
/// use insult_arena_mcp::experimental::sensei::Sensei;
/// use insult_arena_mcp::InsultBank;
///
/// // 1. Create a Sensei (Skill 0.5 = 50% chance to defend)
/// let sensei = Sensei::new(0.5);
/// let bank = InsultBank::new();
///
/// // 2. Sensei attacks
/// let insult = sensei.attack(&bank);
/// println!("Sensei says: {}", insult);
///
/// // 3. Sensei defends
/// let comeback = sensei.defend(&bank, &insult);
/// println!("Sensei responds: {}", comeback);
/// ```
#[derive(Debug, Clone)]
pub struct Sensei {
    /// Skill level (0.0 to 1.0).
    skill: f64,
}

impl Sensei {
    /// Creates a new Sensei with the given skill level.
    ///
    /// # Panics
    ///
    /// Does not panic. `NaN` values are treated as `0.0`.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(skill: f64) -> Self {
        // Clamp skill between 0.0 and 1.0. Handle NaN by treating as 0.0.
        let skill = if skill.is_nan() { 0.0 } else { skill };
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn sensei_handles_floating_point_edge_cases() {
        let bank = InsultBank::new();
        let insult = "You fight like a dairy farmer!";

        // Case 1: NaN -> 0.0 (Fails)
        // Regression test for "gen_bool panic"
        let sensei = Sensei::new(f64::NAN);
        let response = sensei.defend(&bank, insult);
        assert!(
            bank.check_comeback(insult, &response).is_none(),
            "NaN should be treated as 0.0 skill (always fail)"
        );

        // Case 2: Infinity -> 1.0 (Succeeds)
        let sensei = Sensei::new(f64::INFINITY);
        let response = sensei.defend(&bank, insult);
        assert!(
            bank.check_comeback(insult, &response).is_some(),
            "Infinity should be treated as 1.0 skill (always succeed)"
        );

        // Case 3: Neg Infinity -> 0.0 (Fails)
        let sensei = Sensei::new(f64::NEG_INFINITY);
        let response = sensei.defend(&bank, insult);
        assert!(
            bank.check_comeback(insult, &response).is_none(),
            "Neg Infinity should be treated as 0.0 skill (always fail)"
        );

        // Case 4: > 1.0 -> 1.0 (Succeeds)
        let sensei = Sensei::new(1.5);
        let response = sensei.defend(&bank, insult);
        assert!(
            bank.check_comeback(insult, &response).is_some(),
            "1.5 should be clamped to 1.0"
        );

        // Case 5: < 0.0 -> 0.0 (Fails)
        let sensei = Sensei::new(-0.5);
        let response = sensei.defend(&bank, insult);
        assert!(
            bank.check_comeback(insult, &response).is_none(),
            "-0.5 should be clamped to 0.0"
        );
    }
}
