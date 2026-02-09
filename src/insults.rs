//! Classic insult/response pairs from the Monkey Island series.
//!
//! # Memory Efficiency
//!
//! The [`InsultBank`] stores all insults in static memory (`&'static str`), meaning
//! there is zero heap allocation when creating the bank or accessing the insults.
//! This ensures the server remains lightweight even under load.
//!
//! String comparisons are performed using iterators to avoid allocating temporary
//! strings for normalization.

use serde::{Deserialize, Serialize};

/// A matched insult and comeback pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InsultPair {
    /// The insult thrown by the attacker.
    pub insult: &'static str,
    /// The correct comeback that defeats the insult.
    pub comeback: &'static str,
}

const MAX_SEARCH_QUERY_LENGTH: usize = 128;

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

/// Helper to check if two strings match after normalization.
///
/// Normalization involves:
/// 1. Ignoring non-alphanumeric characters.
/// 2. Case-insensitivity.
///
/// This implementation avoids heap allocations.
fn normalized_eq(a: &str, b: &str) -> bool {
    let a_iter = a
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase);

    let b_iter = b
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase);

    a_iter.eq(b_iter)
}

/// Helper to check if a haystack contains a needle, ignoring case.
///
/// Used only for testing.
#[cfg(test)]
fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    let needle_chars: Vec<char> = needle.chars().flat_map(char::to_lowercase).collect();
    contains_ignore_case_char_slice(haystack, &needle_chars)
}

/// Helper to check if a haystack contains a needle (pre-normalized as char slice).
///
/// This avoids re-normalizing the needle for every position in the haystack.
fn contains_ignore_case_char_slice(haystack: &str, needle: &[char]) -> bool {
    if needle.is_empty() {
        return true;
    }

    let mut haystack_iter = haystack.chars().flat_map(char::to_lowercase);

    loop {
        let mut check_iter = haystack_iter.clone();

        let mut matched = true;
        for &n in needle {
            if check_iter.next() != Some(n) {
                matched = false;
                break;
            }
        }

        if matched {
            return true;
        }

        if haystack_iter.next().is_none() {
            break;
        }
    }

    false
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
        self.pairs.iter().find(|pair| {
            normalized_eq(pair.insult, insult) && normalized_eq(pair.comeback, comeback)
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
        self.pairs
            .iter()
            .find(|pair| normalized_eq(pair.insult, insult))
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
        // Hardening: Limit query length to prevent DoS via massive allocation.
        // We take the first MAX_SEARCH_QUERY_LENGTH chars.
        let query_chars: Vec<char> = query
            .chars()
            .take(MAX_SEARCH_QUERY_LENGTH)
            .flat_map(char::to_lowercase)
            .collect();
        self.pairs
            .iter()
            .filter(|pair| contains_ignore_case_char_slice(pair.insult, &query_chars))
            .collect()
    }

    /// Returns a masked version of the string where only the first letter of each word is visible.
    /// Used for hints.
    ///
    /// # Examples
    ///
    /// ```
    /// use insult_arena_mcp::InsultBank;
    /// let hint = InsultBank::get_hint_masked("How appropriate. You fight like a cow!");
    /// assert_eq!(hint, "H__ a__________. Y__ f____ l___ a c__!");
    /// ```
    #[must_use]
    pub fn get_hint_masked(text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let mut words = text.split_whitespace();

        if let Some(first_word) = words.next() {
            Self::mask_word_into(&mut result, first_word);

            for word in words {
                result.push(' ');
                Self::mask_word_into(&mut result, word);
            }
        }

        result
    }

    fn mask_word_into(output: &mut String, word: &str) {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            output.push(first);
            for c in chars {
                if c.is_alphabetic() {
                    output.push('_');
                } else {
                    output.push(c);
                }
            }
        }
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

    #[test]
    fn check_comeback_regression_complex_inputs() {
        let bank = InsultBank::new();

        // 1. Unicode casing regression check (if applicable, English insults are ASCII but good to be safe)
        // Note: 'İ' lowercases to 'i' + dot in some locales, but here we just want to ensure consistency.
        // Given we deal with English insults, we test standard variation.

        let result = bank.check_comeback(
            "You FIGHT like a dairy FARMER!",
            "how appropriate. you fight like a cow!",
        );
        assert!(result.is_some(), "Standard mixed case match failed");

        // 2. Heavy punctuation
        let result = bank.check_comeback(
            "You fight like a dairy farmer!?!",
            "How... appropriate... You fight like a cow!!!",
        );
        assert!(result.is_some(), "Heavy punctuation match failed");

        // 3. No punctuation
        let result = bank.check_comeback(
            "you fight like a dairy farmer",
            "how appropriate you fight like a cow",
        );
        assert!(result.is_some(), "No punctuation match failed");

        // 4. Embedded non-alphanumeric that splits words?
        // "dairy-farmer" vs "dairy farmer".
        // Old normalize: "dairy-farmer" -> "dairyfarmer". "dairy farmer" -> "dairyfarmer".
        // New normalized_eq should handle this too.
        let result = bank.check_comeback(
            "You fight like a dairy-farmer!",
            "How appropriate. You fight like a cow!",
        );
        assert!(result.is_some(), "Hyphenated word match failed");
    }

    #[test]
    fn test_contains_ignore_case() {
        // Exact match
        assert!(contains_ignore_case("Hello World", "Hello"));
        assert!(contains_ignore_case("Hello World", "World"));
        assert!(contains_ignore_case("Hello World", "Hello World"));

        // Case insensitive
        assert!(contains_ignore_case("Hello World", "hello"));
        assert!(contains_ignore_case("Hello World", "WORLD"));
        assert!(contains_ignore_case("HeLLo", "hell"));

        // Empty needle
        assert!(contains_ignore_case("Anything", ""));
        assert!(contains_ignore_case("", ""));

        // No match
        assert!(!contains_ignore_case("Hello World", "Goodbye"));
        assert!(!contains_ignore_case("Short", "LongerString"));

        // Special characters (should be preserved)
        assert!(contains_ignore_case("Hello!", "!"));
        assert!(contains_ignore_case("A+B=C", "+b="));
    }
}

#[cfg(test)]
mod sentry_tests {
    use super::*;

    #[test]
    fn test_get_hint_masked_edge_cases() {
        let cases = vec![
            ("", ""),
            ("Hello", "H____"),
            ("Don't", "D__'_"),
            ("a", "a"),
            ("!!!", "!!!"),
            ("Hello World", "H____ W____"),
            ("   Spaces   ", "S_____"),
        ];

        for (input, expected) in cases {
            assert_eq!(
                InsultBank::get_hint_masked(input),
                expected,
                "Failed for input: '{input}'"
            );
        }
    }

    #[test]
    fn test_normalized_eq_edge_cases() {
        assert!(normalized_eq("", ""), "Empty strings should match");
        assert!(
            normalized_eq("!!!", "???"),
            "Only symbols should match empty logic (both become empty)"
        );
        assert!(normalized_eq("a", "A"), "Case insensitive");
        assert!(normalized_eq("foo-bar", "foobar"), "Symbols ignored");
        assert!(
            !normalized_eq("abc", "def"),
            "Different text should not match"
        );
    }
}

#[cfg(test)]
mod sentry_robustness_tests {
    use super::*;

    #[test]
    fn normalized_eq_unicode_behavior() {
        // Verify that unicode characters are processed safely
        // They should match themselves case-insensitively
        assert!(normalized_eq("Ño", "ño"));

        // But they should NOT match their base characters (Rust default behavior)
        assert!(!normalized_eq("Ño", "No"));

        // Verify emojis are ignored (not alphanumeric)
        // "Cool 😎" -> "cool"
        // "Cool" -> "cool"
        assert!(normalized_eq("Cool 😎", "Cool"));
    }

    #[test]
    fn check_comeback_handles_garbage_input() {
        let bank = InsultBank::new();

        // Garbage insult that shouldn't match anything in the bank
        let result = bank.check_comeback("@@@@", "####");
        assert!(result.is_none());

        // Garbage comeback for a real insult
        // "You fight like a dairy farmer!" (valid)
        // "####" (invalid)
        // "####" normalizes to ""
        // "How appropriate..." normalizes to "howappropriate..."
        // "" != "howappropriate..."
        let result = bank.check_comeback("You fight like a dairy farmer!", "####");
        assert!(result.is_none());
    }

    #[test]
    fn search_insults_robustness() {
        let bank = InsultBank::new();

        // 1. Empty query returns ALL insults
        // Implementation detail: empty needle matches everything.
        let results = bank.search_insults("");
        assert_eq!(results.len(), 16);

        // 2. Query with only punctuation
        // "!!!" -> normalized to "!!!" (because search query does NOT filter alphanumeric, only lowercase)
        // Insults contain "!" so it might match.
        // "You fight like a dairy farmer!" contains "!" at the end.
        let results = bank.search_insults("!");
        assert!(!results.is_empty());

        // 3. Query with non-matching chars
        let results = bank.search_insults("zxzxzx");
        assert!(results.is_empty());

        // 4. Unicode search
        // "beggar"
        let results = bank.search_insults("BEGGAR");
        assert_eq!(results.len(), 1);
        assert!(results[0].insult.contains("beggar"));

        // 5. Long query (DoS protection)
        // MAX_SEARCH_QUERY_LENGTH = 128
        // If we send a huge string, it should be truncated and search should proceed safely (likely returning nothing or partial match).
        let long_query = "a".repeat(1000);
        let results = bank.search_insults(&long_query);
        assert!(results.is_empty());
    }
}
