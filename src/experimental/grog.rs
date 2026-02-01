//! Grog module for simulating drunkenness.

use rand::Rng;

/// Mixes the text with grog, simulating drunkenness.
///
/// Applies various transformations:
/// - Replaces 's' with 'sh' occasionally
/// - Multiplies 'r's occasionally
/// - Drops 'g' from 'ing'
/// - Inserts hiccups
#[must_use]
pub fn mix(text: &str) -> String {
    let mut rng = rand::thread_rng();
    let mut result = String::with_capacity(text.len() * 2);

    let words: Vec<&str> = text.split(' ').collect();

    for (i, word) in words.iter().enumerate() {
        // Randomly insert hiccup before word
        if rng.gen_bool(0.15) {
            result.push_str("*hic* ");
        }

        // Process the word
        let mut processed = word.to_string();

        // "ing" -> "in'"
        if processed.len() > 3 && processed.ends_with("ing") {
            processed.truncate(processed.len() - 1);
            processed.push('\'');
        } else if processed.ends_with("ing.") {
            processed = processed.replace("ing.", "in'.");
        } else if processed.ends_with("ing!") {
            processed = processed.replace("ing!", "in'!");
        } else if processed.ends_with("ing?") {
            processed = processed.replace("ing?", "in'?");
        } else if processed.ends_with("ing,") {
            processed = processed.replace("ing,", "in',");
        }

        // Character replacements
        let mut chars = processed.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                's' | 'S' => {
                    result.push(c);
                    // Add 'h' if next char isn't already 'h'
                    if rng.gen_bool(0.4) && !matches!(chars.peek(), Some('h' | 'H')) {
                        result.push('h');
                    }
                }
                'r' | 'R' => {
                    result.push(c);
                    // Stutter 'r'
                    if rng.gen_bool(0.4) {
                        result.push(c.to_ascii_lowercase());
                    }
                }
                _ => result.push(c),
            }
        }

        if i < words.len() - 1 {
            result.push(' ');
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mix_basic() {
        let input = "Testing the grog system";
        let output = mix(input);
        assert!(!output.is_empty());
        // Since it's random, we can't assert exact output, but we can check it didn't crash
    }

    #[test]
    fn test_mix_preserves_lengthy_text() {
        let input = "This is a very long sentence to test that we don't drop words entirely from the output when mixing.";
        let output = mix(input);
        assert!(output.len() >= input.len()); // Should generally be longer or same due to insertions
    }
}
