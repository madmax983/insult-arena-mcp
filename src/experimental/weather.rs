use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

/// Represents the current environmental conditions of the arena.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WeatherCondition {
    /// A clear night, perfect for dueling.
    #[default]
    Clear,
    /// Thick fog that obscures vision (and potentially hints).
    Fog,
    /// A raging storm that excites the crowd but makes it hard to hear.
    Storm,
    /// A sweltering heatwave that makes everyone sluggish.
    Heatwave,
}

/// Manages the arena's weather and its effects on gameplay/audience.
#[derive(Debug, Clone)]
pub struct WeatherSystem {
    pub current: WeatherCondition,
}

impl Default for WeatherSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl WeatherSystem {
    /// Creates a new `WeatherSystem`, defaulting to Clear weather.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            current: WeatherCondition::Clear,
        }
    }

    /// Randomizes the current weather condition.
    pub fn randomize(&mut self) {
        let conditions = [
            WeatherCondition::Clear,
            WeatherCondition::Fog,
            WeatherCondition::Storm,
            WeatherCondition::Heatwave,
        ];
        // We unwrap_or(&Clear) just to be safe, though choose shouldn't fail on a non-empty slice.
        self.current = conditions
            .choose(&mut rand::thread_rng())
            .unwrap_or(&WeatherCondition::Clear)
            .clone();
    }

    /// Returns a flavor text description of the current weather.
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self.current {
            WeatherCondition::Clear => "The stars are out. A perfect night for a duel.",
            WeatherCondition::Fog => "A thick pea-soup fog rolls in. You can barely see your hand in front of your face.",
            WeatherCondition::Storm => "Thunder crashes overhead! Rain lashes the deck!",
            WeatherCondition::Heatwave => "The air is stiflingly hot. Sweat drips from your brow.",
        }
    }

    /// Modifies hype changes based on the weather.
    ///
    /// * `Storm` amplifies crowd reactions (x1.5).
    /// * `Heatwave` dampens crowd reactions (x0.5).
    /// * `Fog` and `Clear` have no effect.
    #[must_use]
    pub const fn apply_hype_modifier(&self, hype_change: i32) -> i32 {
        match self.current {
            WeatherCondition::Storm => {
                // Integer math: x1.5 is roughly (x * 3) / 2
                (hype_change * 3) / 2
            }
            WeatherCondition::Heatwave => hype_change / 2,
            _ => hype_change,
        }
    }

    /// Determines if hints (via the Parrot) should be obscured.
    /// Returns true if the weather prevents hints.
    #[must_use]
    pub const fn obscures_hints(&self) -> bool {
        matches!(self.current, WeatherCondition::Fog)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn weather_initializes_clear() {
        let weather = WeatherSystem::new();
        assert_eq!(weather.current, WeatherCondition::Clear);
        assert_eq!(weather.description(), "The stars are out. A perfect night for a duel.");
    }

    #[test]
    fn randomize_changes_weather() {
        let mut weather = WeatherSystem::new();
        // It's random, so we can't guarantee a change in one call,
        // but we can try a few times and ensure it's valid.
        for _ in 0..10 {
            weather.randomize();
            let desc = weather.description();
            assert!(!desc.is_empty());
        }
    }

    #[test]
    fn hype_modifier_works() {
        let mut weather = WeatherSystem::new();

        // Clear - no change
        weather.current = WeatherCondition::Clear;
        assert_eq!(weather.apply_hype_modifier(10), 10);
        assert_eq!(weather.apply_hype_modifier(-10), -10);

        // Storm - amplified
        weather.current = WeatherCondition::Storm;
        assert_eq!(weather.apply_hype_modifier(10), 15); // 10 * 1.5 = 15
        assert_eq!(weather.apply_hype_modifier(-10), -15);

        // Heatwave - dampened
        weather.current = WeatherCondition::Heatwave;
        assert_eq!(weather.apply_hype_modifier(10), 5); // 10 * 0.5 = 5
        assert_eq!(weather.apply_hype_modifier(-10), -5);
    }

    #[test]
    fn fog_obscures_hints() {
        let mut weather = WeatherSystem::new();
        weather.current = WeatherCondition::Clear;
        assert!(!weather.obscures_hints());

        weather.current = WeatherCondition::Fog;
        assert!(weather.obscures_hints());
    }
}
