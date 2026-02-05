use std::collections::HashMap;

/// Manages the intoxication levels of players.
#[derive(Debug, Default)]
pub struct GrogSystem {
    /// Maps session ID to grog level (0-5).
    drinkers: HashMap<String, u8>,
}

impl GrogSystem {
    /// Creates a new `GrogSystem`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            drinkers: HashMap::new(),
        }
    }

    /// Increases the grog level for a session.
    /// Max level is 5.
    pub fn drink(&mut self, session_id: String) -> u8 {
        let level = self.drinkers.entry(session_id).or_insert(0);
        if *level < 5 {
            *level += 1;
        }
        *level
    }

    /// Decreases the grog level for a session.
    pub fn sober_up(&mut self, session_id: &str) -> u8 {
        self.drinkers.get_mut(session_id).map_or(0, |level| {
            if *level > 0 {
                *level -= 1;
            }
            *level
        })
    }

    /// Gets the current grog level.
    #[must_use]
    pub fn get_level(&self, session_id: &str) -> u8 {
        *self.drinkers.get(session_id).unwrap_or(&0)
    }

    /// Modifies text based on the session's grog level.
    ///
    /// - Level 0: No change.
    /// - Level 1: Appends " *hic*".
    /// - Level 2: 's' -> 'sh'.
    /// - Level 3: 'r' -> 'rr'.
    /// - Level 4: Prepends "Arr! ".
    /// - Level 5: SCREAMING (Upper case).
    #[must_use]
    pub fn slur(&self, text: &str, session_id: &str) -> String {
        let level = self.get_level(session_id);
        if level == 0 {
            return text.to_string();
        }

        let mut slurred = text.to_string();

        if level >= 5 {
            slurred = slurred.to_uppercase();
        }

        if level >= 2 {
            // "s" -> "sh", but avoid double replacement if possible
            // Simple replace is fine for fun
            slurred = slurred.replace('s', "sh").replace('S', "SH");
        }

        if level >= 3 {
            slurred = slurred.replace('r', "rr").replace('R', "RR");
        }

        if level >= 4 {
            slurred = format!("Arr! {slurred}");
        }

        if level >= 1 {
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
    fn drink_increases_level() {
        let mut grog = GrogSystem::new();
        let session = "pirate1".to_string();

        assert_eq!(grog.drink(session.clone()), 1);
        assert_eq!(grog.drink(session.clone()), 2);
        assert_eq!(grog.drink(session.clone()), 3);
        assert_eq!(grog.drink(session.clone()), 4);
        assert_eq!(grog.drink(session.clone()), 5);
        assert_eq!(grog.drink(session), 5); // Cap at 5
    }

    #[test]
    fn sober_up_decreases_level() {
        let mut grog = GrogSystem::new();
        let session = "pirate1".to_string();

        grog.drink(session.clone()); // 1
        grog.drink(session.clone()); // 2

        assert_eq!(grog.sober_up(&session), 1);
        assert_eq!(grog.sober_up(&session), 0);
        assert_eq!(grog.sober_up(&session), 0); // Min at 0
    }

    #[test]
    fn slur_applies_modifiers() {
        let mut grog = GrogSystem::new();
        let session = "pirate1".to_string();
        let text = "smart";

        // Level 0
        assert_eq!(grog.slur(text, &session), "smart");

        // Level 1: Append *hic*
        grog.drink(session.clone());
        assert_eq!(grog.slur(text, &session), "smart *hic*");

        // Level 2: s -> sh
        grog.drink(session.clone());
        assert_eq!(grog.slur(text, &session), "shmart *hic*");

        // Level 3: r -> rr
        grog.drink(session.clone());
        assert_eq!(grog.slur(text, &session), "shmarrt *hic*");

        // Level 4: Prepend Arr!
        grog.drink(session.clone());
        assert_eq!(grog.slur(text, &session), "Arr! shmarrt *hic*");

        // Level 5: Uppercase
        grog.drink(session.clone());
        // Original "smart" -> Upper "SMART" -> "SHMART" -> "SHMARRT" -> "Arr! SHMARRT *hic*"
        assert_eq!(grog.slur(text, &session), "Arr! SHMARRT *hic*");
    }
}
