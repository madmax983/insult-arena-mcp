use rand::Rng;
use std::collections::HashMap;

/// The Grog System manages player intoxication levels and text slurring.
///
/// "Grog! It's the secret ingredient!"
#[derive(Debug, Default)]
pub struct GrogSystem {
    intoxication: HashMap<String, u8>,
}

impl GrogSystem {
    /// Creates a new `GrogSystem`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            intoxication: HashMap::new(),
        }
    }

    /// Drinks grog, increasing intoxication level for the session.
    /// Returns the new level.
    pub fn drink_grog(&mut self, session_id: &str) -> u8 {
        let entry = self.intoxication.entry(session_id.to_string()).or_insert(0);
        *entry = entry.saturating_add(1).min(10); // Max level 10
        *entry
    }

    /// Gets the intoxication level for a session.
    #[must_use]
    pub fn get_level(&self, session_id: &str) -> u8 {
        *self.intoxication.get(session_id).unwrap_or(&0)
    }

    /// Slurs the text based on intoxication level.
    #[must_use]
    pub fn slur_text(&self, text: &str, level: u8) -> String {
        if level == 0 {
            return text.to_string();
        }

        let mut slurred = String::with_capacity(text.len() * 2);
        let mut rng = rand::thread_rng();

        for c in text.chars() {
            // High intoxication: 's' -> 'sh'
            if level >= 3 && (c == 's' || c == 'S') {
                slurred.push(c);
                slurred.push('h');
                continue;
            }

            // Extreme intoxication: 'r' -> 'rr'
            if level >= 6 && (c == 'r' || c == 'R') {
                slurred.push(c);
                slurred.push(c.to_ascii_lowercase());
                continue;
            }

            slurred.push(c);

            // Random hiccups
            if c.is_whitespace() && rng.gen_bool(0.05 * f64::from(level)) {
                slurred.push_str("*hic* ");
            }
        }

        // Append a final hiccup if very drunk
        if level >= 5 && !slurred.ends_with("*hic*") && rng.gen_bool(0.5) {
            slurred.push_str(" *hic*");
        }

        slurred
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn drink_grog_increases_level() {
        let mut grog = GrogSystem::new();
        assert_eq!(grog.drink_grog("p1"), 1);
        assert_eq!(grog.drink_grog("p1"), 2);
        assert_eq!(grog.get_level("p1"), 2);
    }

    #[test]
    fn max_intoxication_is_capped() {
        let mut grog = GrogSystem::new();
        for _ in 0..20 {
            grog.drink_grog("p1");
        }
        assert_eq!(grog.get_level("p1"), 10);
    }

    #[test]
    fn slur_text_does_nothing_when_sober() {
        let grog = GrogSystem::new();
        let text = "You fight like a dairy farmer!";
        assert_eq!(grog.slur_text(text, 0), text);
    }

    #[test]
    fn slur_text_adds_sh_when_tipsy() {
        let grog = GrogSystem::new();
        let text = "So you want to be a sword master?";
        let slurred = grog.slur_text(text, 3);

        assert!(slurred.contains("Sh") || slurred.contains("sh"));
    }

    #[test]
    fn slur_text_adds_hiccups() {
        let grog = GrogSystem::new();
        // Max level to guarantee hiccups (probability 0.05 * 10 = 0.5 per space)
        // With enough spaces, it should happen.
        let text = "one two three four five six seven eight nine ten";

        // Try multiple times to avoid flakiness
        let mut found_hic = false;
        for _ in 0..10 {
            let slurred = grog.slur_text(text, 10);
            if slurred.contains("*hic*") {
                found_hic = true;
                break;
            }
        }
        assert!(found_hic, "Should eventually hiccup at max level");
    }
}
