//! AI Logic for the Dojo opponent.
//!
//! The Sensei is a computer-controlled opponent that can duel the player in the Dojo.
//! It has a configurable skill level that determines its probability of making a correct move.

use crate::InsultBank;
use rand::Rng;

/// The Sensei (AI) opponent in the Dojo.
///
/// The Sensei is a simulated opponent with a configurable skill level.
/// - **Low Skill**: Frequently makes mistakes.
/// - **High Skill**: Almost never misses a comeback.
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
    /// # Arguments
    ///
    /// * `skill` - A value between `0.0` (total novice) and `1.0` (perfect master).
    ///   - Values outside this range are clamped.
    ///   - `NaN` is treated as `0.0`.
    ///
    /// # Panics
    ///
    /// Does not panic. Handles invalid floats gracefully.
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
    ///
    /// The Sensei always picks a valid insult from the bank.
    ///
    /// # Examples
    ///
    /// ```
    /// use insult_arena_mcp::experimental::sensei::Sensei;
    /// use insult_arena_mcp::InsultBank;
    ///
    /// let sensei = Sensei::new(0.5);
    /// let bank = InsultBank::new();
    /// let insult = sensei.attack(&bank);
    /// assert!(!insult.is_empty());
    /// ```
    #[must_use]
    pub fn attack(&self, bank: &InsultBank) -> String {
        // Sensei always picks a valid insult
        bank.random_insult().insult.to_string()
    }

    /// Decides on a comeback response.
    ///
    /// The Sensei's success rate depends on their skill level.
    /// - If the skill check passes, they return the correct comeback.
    /// - If the skill check fails, they return a generic wrong answer.
    ///
    /// # Examples
    ///
    /// ```
    /// use insult_arena_mcp::experimental::sensei::Sensei;
    /// use insult_arena_mcp::InsultBank;
    ///
    /// let sensei = Sensei::new(1.0); // Perfect skill
    /// let bank = InsultBank::new();
    ///
    /// let comeback = sensei.defend(&bank, "You fight like a dairy farmer!");
    /// assert_eq!(comeback, "How appropriate. You fight like a cow!");
    /// ```
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
