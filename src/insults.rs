//! Classic insult/response pairs from the Monkey Island series.

use serde::{Deserialize, Serialize};

/// A matched insult and comeback pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InsultPair {
    /// The insult thrown by the attacker.
    pub insult: &'static str,
    /// The correct comeback that defeats the insult.
    pub comeback: &'static str,
}

const CLASSIC_INSULTS: &[InsultPair] = &[
    InsultPair {
        insult: "You fight like a dairy farmer!",
        comeback: "How appropriate. You fight like a cow!",
    },
    InsultPair {
        insult: "This is the END for you, you gutter-crawling cur!",
        comeback: "And I've got a little TIP for you, get the POINT?",
    },
    InsultPair {
        insult: "I've spoken with apes more polite than you!",
        comeback: "I'm glad to hear you attended your family reunion!",
    },
    InsultPair {
        insult: "Soon you'll be wearing my sword like a shish kebab!",
        comeback: "First you'd better stop waving it like a feather duster.",
    },
    InsultPair {
        insult: "People fall at my feet when they see me coming!",
        comeback: "Even BEFORE they smell your breath?",
    },
    InsultPair {
        insult: "I'm not going to take your insolence sitting down!",
        comeback: "Your hemorrhoids are flaring up again eh?",
    },
    InsultPair {
        insult: "I once owned a dog that was smarter than you.",
        comeback: "He must have taught you everything you know.",
    },
    InsultPair {
        insult: "Nobody's ever drawn blood from me and nobody ever will!",
        comeback: "You run THAT fast?",
    },
    InsultPair {
        insult: "Have you stopped wearing diapers yet?",
        comeback: "Why? Did you want to borrow one?",
    },
    InsultPair {
        insult: "There are no words for how disgusting you are.",
        comeback: "Yes, there are. You just never learned them.",
    },
    InsultPair {
        insult: "You make me want to puke.",
        comeback: "You make me think somebody already did.",
    },
    InsultPair {
        insult: "My handkerchief will wipe up your blood!",
        comeback: "So you got that job as a janitor, after all.",
    },
    InsultPair {
        insult: "I got this scar on my face during a mighty struggle!",
        comeback: "I hope now you've learned to stop picking your nose.",
    },
    InsultPair {
        insult: "I've heard you are a contemptible sneak.",
        comeback: "Too bad no one's ever heard of YOU at all.",
    },
    InsultPair {
        insult: "You're no match for my brains, you poor fool.",
        comeback: "I'd be in real trouble if you ever used them.",
    },
    InsultPair {
        insult: "You have the manners of a beggar.",
        comeback: "I wanted to make sure you'd feel comfortable with me.",
    },
];

/// Helper to normalize strings for forgiving matching.
/// Removes non-alphanumeric characters and converts to lowercase.
fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

/// Bank of classic insults for sword fighting.
///
/// Stores insults in static memory to avoid heap allocation.
///
/// # Examples
///
/// ```
/// use insult_arena_mcp::InsultBank;
///
/// let bank = InsultBank::new();
///
/// // Find the correct response
/// let insult = "You fight like a dairy farmer!";
/// let comeback = bank.find_comeback(insult).unwrap();
/// assert_eq!(comeback, "How appropriate. You fight like a cow!");
///
/// // Check if a user's response is correct (case-insensitive)
/// assert!(bank.check_comeback(insult, "how appropriate. you fight like a cow!").is_some());
/// ```
#[derive(Debug, Clone)]
pub struct InsultBank {
    pairs: &'static [InsultPair],
}

impl InsultBank {
    /// Creates a new insult bank with all classic Monkey Island insults.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pairs: CLASSIC_INSULTS,
        }
    }

    /// Returns all insult pairs.
    #[must_use]
    pub const fn all_pairs(&self) -> &[InsultPair] {
        self.pairs
    }

    /// Gets a random insult for an attacker to use.
    ///
    /// # Panics
    ///
    /// Panics if the insult bank is empty. This should never happen as the
    /// bank is always initialized with the classic Monkey Island insults.
    #[must_use]
    #[allow(clippy::expect_used)] // Invariant: InsultBank is never constructed empty
    pub fn random_insult(&self) -> &InsultPair {
        use rand::seq::SliceRandom;
        self.pairs
            .choose(&mut rand::thread_rng())
            .expect("InsultBank should never be empty")
    }

    /// Checks if a comeback is correct for a given insult.
    /// Returns the matching pair if found.
    /// Normalizes both insult and comeback (lowercase, alphanumeric only) before comparing.
    ///
    /// # Examples
    ///
    /// ```
    /// use insult_arena_mcp::InsultBank;
    /// let bank = InsultBank::new();
    ///
    /// // Exact match
    /// let pair = bank.check_comeback(
    ///     "You fight like a dairy farmer!",
    ///     "How appropriate. You fight like a cow!"
    /// );
    /// assert!(pair.is_some());
    ///
    /// // Case insensitive and punctuation ignored
    /// let pair = bank.check_comeback(
    ///     "YOU FIGHT LIKE A DAIRY FARMER!",
    ///     "how appropriate you fight like a cow"
    /// );
    /// assert!(pair.is_some());
    /// ```
    #[must_use]
    pub fn check_comeback(&self, insult: &str, comeback: &str) -> Option<&InsultPair> {
        let insult_norm = normalize(insult);
        let comeback_norm = normalize(comeback);

        self.pairs.iter().find(|pair| {
            normalize(pair.insult) == insult_norm && normalize(pair.comeback) == comeback_norm
        })
    }

    /// Finds the correct comeback for an insult.
    ///
    /// # Examples
    ///
    /// ```
    /// use insult_arena_mcp::InsultBank;
    /// let bank = InsultBank::new();
    ///
    /// let comeback = bank.find_comeback("You fight like a dairy farmer!");
    /// assert_eq!(comeback, Some("How appropriate. You fight like a cow!"));
    /// ```
    #[must_use]
    pub fn find_comeback(&self, insult: &str) -> Option<&str> {
        let insult_norm = normalize(insult);

        self.pairs
            .iter()
            .find(|pair| normalize(pair.insult) == insult_norm)
            .map(|pair| pair.comeback)
    }

    /// Finds insults that match a partial string (for learning mode).
    ///
    /// # Examples
    ///
    /// ```
    /// use insult_arena_mcp::InsultBank;
    /// let bank = InsultBank::new();
    ///
    /// let results = bank.search_insults("dairy");
    /// assert_eq!(results.len(), 1);
    /// assert!(results[0].insult.contains("dairy farmer"));
    /// ```
    #[must_use]
    pub fn search_insults(&self, query: &str) -> Vec<&InsultPair> {
        let query_lower = query.to_lowercase();
        self.pairs
            .iter()
            .filter(|pair| pair.insult.to_lowercase().contains(&query_lower))
            .collect()
    }
}

impl Default for InsultBank {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insult_bank_has_16_classic_insults() {
        let bank = InsultBank::new();
        assert_eq!(bank.all_pairs().len(), 16);
    }

    #[test]
    fn dairy_farmer_comeback_is_cow() {
        let bank = InsultBank::new();
        let comeback = bank.find_comeback("You fight like a dairy farmer!");
        assert_eq!(comeback, Some("How appropriate. You fight like a cow!"));
    }

    #[test]
    fn check_comeback_matches_correct_pair() {
        let bank = InsultBank::new();
        let result = bank.check_comeback(
            "You fight like a dairy farmer!",
            "How appropriate. You fight like a cow!",
        );
        assert!(result.is_some());
    }

    #[test]
    fn check_comeback_case_insensitive() {
        let bank = InsultBank::new();
        let result = bank.check_comeback(
            "YOU FIGHT LIKE A DAIRY FARMER!",
            "how appropriate. you fight like a cow!",
        );
        assert!(result.is_some());
    }

    #[test]
    fn check_comeback_wrong_response_returns_none() {
        let bank = InsultBank::new();
        let result = bank.check_comeback("You fight like a dairy farmer!", "No u");
        assert!(result.is_none());
    }

    #[test]
    fn random_insult_returns_valid_pair() {
        let bank = InsultBank::new();
        let pair = bank.random_insult();
        assert!(!pair.insult.is_empty());
        assert!(!pair.comeback.is_empty());
    }

    #[test]
    fn search_insults_finds_matches() {
        let bank = InsultBank::new();
        let results = bank.search_insults("dairy");
        assert_eq!(results.len(), 1);
        assert!(results[0].insult.contains("dairy farmer"));
    }

    #[test]
    fn beggar_manners_comeback_exact_match() {
        let bank = InsultBank::new();
        let comeback = bank.find_comeback("You have the manners of a beggar.");
        assert_eq!(
            comeback,
            Some("I wanted to make sure you'd feel comfortable with me.")
        );
    }

    #[test]
    fn punctuation_is_ignored() {
        let bank = InsultBank::new();

        // Without period should SUCCEED now
        let result = bank.check_comeback(
            "You have the manners of a beggar.",
            "I wanted to make sure you'd feel comfortable with me",
        );
        assert!(result.is_some(), "Missing period should match");

        // Trailing space should succeed
        let result = bank.check_comeback(
            "You have the manners of a beggar.",
            "I wanted to make sure you'd feel comfortable with me. ",
        );
        assert!(result.is_some(), "Trailing whitespace should be ignored");

        // Wrong comeback for insult should fail
        let result = bank.check_comeback(
            "You have the manners of a beggar.",
            "How appropriate. You fight like a cow!",
        );
        assert!(result.is_none(), "Wrong comeback should not match");
    }

    #[test]
    fn check_comeback_forgiving() {
        let bank = InsultBank::new();

        // Missing punctuation
        let result = bank.check_comeback(
            "You have the manners of a beggar.",
            "I wanted to make sure youd feel comfortable with me",
        );
        assert!(
            result.is_some(),
            "Should match even with missing punctuation"
        );

        // Extra punctuation
        let result = bank.check_comeback(
            "You have the manners of a beggar.",
            "I wanted to make sure you'd feel comfortable with me!!!",
        );
        assert!(result.is_some(), "Should match even with extra punctuation");

        // Mixed case and punctuation
        let result = bank.check_comeback(
            "You have the manners of a beggar.",
            "i WANTED to make sure youd feel COMFORTABLE with me...",
        );
        assert!(
            result.is_some(),
            "Should match regardless of case and punctuation"
        );
    }
}
