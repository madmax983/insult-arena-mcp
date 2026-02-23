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
    /// A pre-computed hint for the comeback (masked string).
    ///
    /// The hint is masked so that only the first letter of each word is visible.
    /// This field is skipped during serialization to avoid bloating the API response.
    #[serde(default, skip)]
    pub hint: &'static str,
}

const MAX_SEARCH_QUERY_LENGTH: usize = 128;

const CLASSIC_INSULTS: &[InsultPair] = &[
    InsultPair {
        insult: "You fight like a dairy farmer!",
        comeback: "How appropriate. You fight like a cow!",
        hint: "H__ a__________. Y__ f____ l___ a c__!",
    },
    InsultPair {
        insult: "This is the END for you, you gutter-crawling cur!",
        comeback: "And I've got a little TIP for you, get the POINT?",
        hint: "A__ I'__ g__ a l_____ T__ f__ y__, g__ t__ P____?",
    },
    InsultPair {
        insult: "I've spoken with apes more polite than you!",
        comeback: "I'm glad to hear you attended your family reunion!",
        hint: "I'_ g___ t_ h___ y__ a_______ y___ f_____ r______!",
    },
    InsultPair {
        insult: "Soon you'll be wearing my sword like a shish kebab!",
        comeback: "First you'd better stop waving it like a feather duster.",
        hint: "F____ y__'_ b_____ s___ w_____ i_ l___ a f______ d_____.",
    },
    InsultPair {
        insult: "People fall at my feet when they see me coming!",
        comeback: "Even BEFORE they smell your breath?",
        hint: "E___ B_____ t___ s____ y___ b_____?",
    },
    InsultPair {
        insult: "I'm not going to take your insolence sitting down!",
        comeback: "Your hemorrhoids are flaring up again eh?",
        hint: "Y___ h__________ a__ f______ u_ a____ e_?",
    },
    InsultPair {
        insult: "I once owned a dog that was smarter than you.",
        comeback: "He must have taught you everything you know.",
        hint: "H_ m___ h___ t_____ y__ e_________ y__ k___.",
    },
    InsultPair {
        insult: "Nobody's ever drawn blood from me and nobody ever will!",
        comeback: "You run THAT fast?",
        hint: "Y__ r__ T___ f___?",
    },
    InsultPair {
        insult: "Have you stopped wearing diapers yet?",
        comeback: "Why? Did you want to borrow one?",
        hint: "W__? D__ y__ w___ t_ b_____ o__?",
    },
    InsultPair {
        insult: "There are no words for how disgusting you are.",
        comeback: "Yes, there are. You just never learned them.",
        hint: "Y__, t____ a__. Y__ j___ n____ l______ t___.",
    },
    InsultPair {
        insult: "You make me want to puke.",
        comeback: "You make me think somebody already did.",
        hint: "Y__ m___ m_ t____ s_______ a______ d__.",
    },
    InsultPair {
        insult: "My handkerchief will wipe up your blood!",
        comeback: "So you got that job as a janitor, after all.",
        hint: "S_ y__ g__ t___ j__ a_ a j______, a____ a__.",
    },
    InsultPair {
        insult: "I got this scar on my face during a mighty struggle!",
        comeback: "I hope now you've learned to stop picking your nose.",
        hint: "I h___ n__ y__'__ l______ t_ s___ p______ y___ n___.",
    },
    InsultPair {
        insult: "I've heard you are a contemptible sneak.",
        comeback: "Too bad no one's ever heard of YOU at all.",
        hint: "T__ b__ n_ o__'_ e___ h____ o_ Y__ a_ a__.",
    },
    InsultPair {
        insult: "You're no match for my brains, you poor fool.",
        comeback: "I'd be in real trouble if you ever used them.",
        hint: "I'_ b_ i_ r___ t______ i_ y__ e___ u___ t___.",
    },
    InsultPair {
        insult: "You have the manners of a beggar.",
        comeback: "I wanted to make sure you'd feel comfortable with me.",
        hint: "I w_____ t_ m___ s___ y__'_ f___ c__________ w___ m_.",
    },
];

/// Helper to check if two strings match after normalization.
///
/// Normalization involves:
/// 1. Ignoring non-alphanumeric characters.
/// 2. Case-insensitivity.
///
/// This implementation avoids heap allocations.
pub fn normalized_eq(a: &str, b: &str) -> bool {
    // ⚡ Bolt Optimization: Fast path for ASCII strings.
    // Avoids UTF-8 decoding overhead and complex iterator state in flat_map(to_lowercase).
    // Uses raw bytes which are faster to iterate and process.
    if a.is_ascii() && b.is_ascii() {
        let a_iter = a
            .bytes()
            .filter(u8::is_ascii_alphanumeric)
            .map(|b| b.to_ascii_lowercase());
        let b_iter = b
            .bytes()
            .filter(u8::is_ascii_alphanumeric)
            .map(|b| b.to_ascii_lowercase());
        return a_iter.eq(b_iter);
    }

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

/// Helper to check if a text matches a normalized ASCII byte slice.
///
/// The `text` is normalized on the fly (alphanumeric only, lowercase)
/// and compared byte-by-byte with `normalized_pattern`.
///
/// Returns true if they match exactly.
fn matches_normalized_ascii(text: &str, normalized_pattern: &[u8]) -> bool {
    let mut text_iter = text
        .bytes()
        .filter(u8::is_ascii_alphanumeric)
        .map(|b| b.to_ascii_lowercase());

    for &p in normalized_pattern {
        if text_iter.next() != Some(p) {
            return false;
        }
    }

    text_iter.next().is_none()
}

/// Helper to check if a haystack contains a needle, ignoring case.
///
/// Used only for testing.
#[cfg(test)]
fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    // For tests, heap allocation is acceptable to keep test code simple
    let needle_chars: Vec<char> = needle.chars().flat_map(char::to_lowercase).collect();
    contains_ignore_case_char_slice(haystack, &needle_chars)
}

/// Helper to check if a haystack contains a needle (pre-normalized as byte slice).
///
/// Optimized for ASCII-only haystack and needle.
/// Avoids `char` decoding overhead.
fn contains_ignore_case_bytes(haystack: &str, needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }

    // 🛡️ SENTRY: Normalize haystack (filter non-alphanumeric) to match search query logic.
    // Buffer size 256 is sufficient for insults (~60 chars).
    let mut buffer = [0u8; 256];
    let mut len = 0;

    for b in haystack.bytes() {
        if b.is_ascii_alphanumeric() && len < 256 {
            buffer[len] = b.to_ascii_lowercase();
            len += 1;
        }
    }
    let haystack_norm = &buffer[..len];

    if haystack_norm.len() < needle.len() {
        return false;
    }

    haystack_norm.windows(needle.len()).any(|w| w == needle)
}

/// Helper to check if a Unicode haystack contains an ASCII needle, ignoring case.
///
/// Avoids allocating a `Vec<char>` for the needle when the query is ASCII.
fn contains_ignore_case_mixed(haystack: &str, needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }

    // 🛡️ SENTRY: Normalize haystack (filter non-alphanumeric).
    // Note: ASCII needle can only match ASCII chars in haystack.
    // So we can just filter for ASCII alphanumeric in haystack too.
    let mut buffer = [0u8; 256];
    let mut len = 0;

    for c in haystack.chars() {
        if c.is_ascii_alphanumeric() && len < 256 {
            buffer[len] = c.to_ascii_lowercase() as u8;
            len += 1;
        }
    }
    let haystack_norm = &buffer[..len];

    if haystack_norm.len() < needle.len() {
        return false;
    }

    haystack_norm.windows(needle.len()).any(|w| w == needle)
}

/// Helper to check if a haystack contains a needle (pre-normalized as char slice).
///
/// This avoids re-normalizing the needle for every position in the haystack.
fn contains_ignore_case_char_slice(haystack: &str, needle: &[char]) -> bool {
    if needle.is_empty() {
        return true;
    }

    // 🛡️ SENTRY: Normalize haystack (filter non-alphanumeric).
    // Buffer for normalized haystack chars.
    let mut buffer = ['\0'; 256];
    let mut len = 0;

    for c in haystack
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
    {
        if len < 256 {
            buffer[len] = c;
            len += 1;
        }
    }
    let haystack_norm = &buffer[..len];

    if haystack_norm.len() < needle.len() {
        return false;
    }

    haystack_norm.windows(needle.len()).any(|w| w == needle)
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
    /// # Arguments
    ///
    /// * `insult` - The insult thrown by the attacker.
    /// * `comeback` - The comeback attempted by the defender.
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
        let pair = self.find_pair(insult)?;

        if normalized_eq(pair.comeback, comeback) {
            Some(pair)
        } else {
            None
        }
    }

    /// Finds the correct comeback for an insult.
    ///
    /// # Arguments
    ///
    /// * `insult` - The insult to find a comeback for.
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
    pub fn find_comeback(&self, insult: &str) -> Option<&'static str> {
        self.find_pair(insult).map(|pair| pair.comeback)
    }

    /// Finds the `InsultPair` matching the insult.
    ///
    /// Returns the canonical pair from the bank if the input insult matches.
    ///
    /// # Arguments
    ///
    /// * `insult` - The insult to search for.
    #[must_use]
    pub fn find_pair(&self, insult: &str) -> Option<&'static InsultPair> {
        // ⚡ Bolt Optimization: Zero-allocation normalization for ASCII inputs.
        // We normalize the user input *once* into a stack buffer, then compare against
        // bank entries using `matches_normalized_ascii`.

        // Buffer size: Max insult length is ~60 chars.
        // 256 is plenty and stack-safe.
        const NORM_BUFFER_SIZE: usize = 256;

        if insult.is_ascii() {
            let mut buffer = [0u8; NORM_BUFFER_SIZE];
            let mut len = 0;

            for b in insult.bytes() {
                if b.is_ascii_alphanumeric() {
                    if len >= NORM_BUFFER_SIZE {
                        // Input is way longer than any valid insult (even normalized).
                        // Fail early.
                        return None;
                    }
                    buffer[len] = b.to_ascii_lowercase();
                    len += 1;
                }
            }
            let snapshot = &buffer[..len];

            return self
                .pairs
                .iter()
                .find(|pair| matches_normalized_ascii(pair.insult, snapshot));
        }

        // Fallback for non-ASCII inputs
        self.pairs
            .iter()
            .find(|pair| normalized_eq(pair.insult, insult))
    }

    /// Finds insults that match a partial string (for learning mode).
    ///
    /// # Arguments
    ///
    /// * `query` - The search query string. Case-insensitive.
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
        // ⚡ Bolt Optimization: Zero-allocation search using stack buffer!
        // Avoiding Vec<char> heap allocation (O(1) alloc) while preserving
        // O(M) normalization cost (vs O(H*M) with lazy iterator).

        // Fixed-size stack buffer sufficient for max query length (128) + expansion.
        // 256 chars = 1KB stack usage, well within safe limits.
        const BUFFER_SIZE: usize = 256;

        // If query is ASCII, we can use a faster byte-based search path.
        if query.is_ascii() {
            let mut buffer = [0u8; BUFFER_SIZE];
            let mut len = 0;

            for b in query.bytes().take(MAX_SEARCH_QUERY_LENGTH) {
                // 🛡️ SENTRY: Filter non-alphanumeric characters to match find_pair logic
                if b.is_ascii_alphanumeric() {
                    if len < BUFFER_SIZE {
                        buffer[len] = b.to_ascii_lowercase();
                        len += 1;
                    } else {
                        break;
                    }
                }
            }
            let query_bytes = &buffer[..len];

            return self
                .pairs
                .iter()
                .filter(|pair| {
                    if pair.insult.is_ascii() {
                        contains_ignore_case_bytes(pair.insult, query_bytes)
                    } else {
                        contains_ignore_case_mixed(pair.insult, query_bytes)
                    }
                })
                .collect();
        }

        let mut buffer = ['\0'; BUFFER_SIZE];
        let mut len = 0;

        for c in query
            .chars()
            .take(MAX_SEARCH_QUERY_LENGTH)
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
        {
            if len < BUFFER_SIZE {
                buffer[len] = c;
                len += 1;
            } else {
                break;
            }
        }
        let query_chars = &buffer[..len];

        self.pairs
            .iter()
            .filter(|pair| contains_ignore_case_char_slice(pair.insult, query_chars))
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

    #[test]
    fn test_find_pair_optimization_edge_cases() {
        let bank = InsultBank::new();

        // 1. Long garbage input - should return None quickly
        let long_input = "a".repeat(500);
        assert!(bank.find_pair(&long_input).is_none());

        // 2. Exact match check
        assert!(bank.find_pair("You fight like a dairy farmer!").is_some());

        // 3. Case insensitive
        assert!(bank.find_pair("you fight like a dairy farmer!").is_some());

        // 4. Punctuation
        assert!(bank.find_pair("you fight like a dairy farmer...").is_some());
    }

    #[test]
    fn test_matches_normalized_ascii() {
        // "abc" normalized
        let pattern = b"abc";
        assert!(matches_normalized_ascii("abc", pattern));
        assert!(matches_normalized_ascii("A B C", pattern));
        assert!(matches_normalized_ascii("...abc...", pattern));
        assert!(!matches_normalized_ascii("ab", pattern));
        assert!(!matches_normalized_ascii("abcd", pattern));
        assert!(!matches_normalized_ascii("xyz", pattern));
    }

    #[test]
    fn test_precomputed_hints_match_legacy_logic() {
        // ⚡ Bolt Verification: Ensure pre-computed hints are correct
        let bank = InsultBank::new();
        for pair in bank.all_pairs() {
            let legacy = InsultBank::get_hint_masked(pair.comeback);
            assert_eq!(
                pair.hint, legacy,
                "Pre-computed hint mismatch for '{}'",
                pair.comeback
            );
        }
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

    #[test]
    fn search_insults_truncation_safety() {
        let bank = InsultBank::new();
        // Create a query that expands significantly.
        // 'ß' expands to "ss".
        // 130 'ß' chars -> 260 's' chars.
        // BUFFER_SIZE is 256.
        // This should not panic.
        let query = "ß".repeat(130);
        let results = bank.search_insults(&query);
        // It shouldn't match anything, but most importantly it shouldn't panic.
        assert!(results.is_empty());
    }
}
