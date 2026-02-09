use crate::insults::InsultBank;
use rand::seq::SliceRandom;

/// A helpful parrot that gives hints for insults.
#[derive(Debug, Clone)]
pub struct Parrot {
    bank: InsultBank,
}

impl Default for Parrot {
    fn default() -> Self {
        Self::new()
    }
}

impl Parrot {
    /// Creates a new Parrot instance.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bank: InsultBank::new(),
        }
    }

    /// Returns a hint for the given insult.
    /// The hint masks all but the first letter of each word in the comeback.
    #[must_use]
    pub fn hint(&self, insult: &str) -> Option<String> {
        let comeback = self.bank.find_comeback(insult)?;
        Some(InsultBank::get_hint_masked(comeback))
    }

    /// SQUAWK! Returns a random pirate sound.
    #[must_use]
    pub fn squawk(&self) -> &'static str {
        let squawks = [
            "Bawk! Pieces of Eight!",
            "Squawk! Polly want a cracker!",
            "Shiver me timbers! Bawk!",
            "Dead men tell no tales! Squawk!",
        ];
        // simple random choice
        squawks.choose(&mut rand::thread_rng()).unwrap_or(&"Bawk!")
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn parrot_initializes() {
        let parrot = Parrot::new();
        let squawk = parrot.squawk();
        assert!(!squawk.is_empty());
    }

    #[test]
    fn hint_masks_correctly() {
        let parrot = Parrot::new();
        // Comeback: "How appropriate. You fight like a cow!"
        let hint = parrot.hint("You fight like a dairy farmer!").unwrap();

        // Expected: "H__ a__________. Y__ f____ l___ a c__!"

        let expected_parts = ["H__", "a__________.", "Y__", "f____", "l___", "a", "c__!"];
        for part in expected_parts {
            assert!(hint.contains(part), "Hint missing part: {part}");
        }

        // Ensure no full words are leaked (except short ones like "a")
        assert!(!hint.contains("How"));
        assert!(!hint.contains("fight"));
        assert!(!hint.contains("cow"));
    }

    #[test]
    fn hint_handles_unknown_insult() {
        let parrot = Parrot::new();
        let hint = parrot.hint("You fight like a turnip!");
        assert!(hint.is_none());
    }

    #[test]
    fn squawk_returns_valid_string() {
        let parrot = Parrot::new();
        let s = parrot.squawk();
        assert!(
            s.contains("Squawk")
                || s.contains("Bawk")
                || s.contains("Shiver")
                || s.contains("Dead")
        );
    }

    #[test]
    fn hint_is_case_insensitive() {
        let parrot = Parrot::new();
        // Use uppercase "DAIRY FARMER"
        let hint = parrot.hint("YOU FIGHT LIKE A DAIRY FARMER!").unwrap();
        // Should find "How appropriate. You fight like a cow!"
        assert!(
            hint.contains("H__"),
            "Hint should be found even with uppercase input"
        );
    }
}
