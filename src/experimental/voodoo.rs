use crate::experimental::weather::WeatherCondition;

/// The Voodoo Doll channels the spirits of the island to transform speech
/// based on the environmental conditions.
pub struct VoodooDoll;

impl VoodooDoll {
    /// Transforms the input text based on the provided weather condition.
    #[must_use]
    pub fn channel(text: &str, weather: &WeatherCondition) -> String {
        match weather {
            WeatherCondition::Clear => text.to_string(),
            WeatherCondition::Fog => Self::apply_fog(text),
            WeatherCondition::Storm => Self::apply_storm(text),
            WeatherCondition::Heatwave => Self::apply_heatwave(text),
        }
    }

    fn apply_fog(text: &str) -> String {
        // Fog makes things uncertain and spooky.
        // "You fight like a dairy farmer!" -> "You... fight... like a... dairy farmer...!"
        let mut result = String::with_capacity(text.len() * 2);
        let words: Vec<&str> = text.split_whitespace().collect();

        for (i, word) in words.iter().enumerate() {
            if i > 0 {
                result.push_str("... ");
            }
            result.push_str(word);
        }

        if !result.ends_with("...") && !result.ends_with('!') {
            result.push_str("...");
        }

        result
    }

    fn apply_storm(text: &str) -> String {
        // Storms are loud and aggressive. Pirate mode!
        let mut s = text.to_uppercase();
        if !s.ends_with('!') {
            s.push('!');
        }
        format!("ARR! {s} THUNDER!")
    }

    fn apply_heatwave(text: &str) -> String {
        // Heatwaves make you sluggish and drunk-sounding.
        // "s" becomes "sh", "r" becomes "w".
        let s = text
            .replace('s', "sh")
            .replace('S', "Sh")
            .replace('r', "w")
            .replace('R', "W");

        format!("{s} *hic*... (so hot)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_weather_leaves_text_alone() {
        let text = "You fight like a dairy farmer!";
        let result = VoodooDoll::channel(text, &WeatherCondition::Clear);
        assert_eq!(result, text);
    }

    #[test]
    fn fog_adds_uncertainty() {
        let text = "I am rubber you are glue";
        let result = VoodooDoll::channel(text, &WeatherCondition::Fog);
        assert_eq!(result, "I... am... rubber... you... are... glue...");
    }

    #[test]
    fn storm_is_loud() {
        let text = "Look behind you";
        let result = VoodooDoll::channel(text, &WeatherCondition::Storm);
        assert_eq!(result, "ARR! LOOK BEHIND YOU! THUNDER!");
    }

    #[test]
    fn heatwave_slurs_speech() {
        let text = "You smell like a rose";
        let result = VoodooDoll::channel(text, &WeatherCondition::Heatwave);
        // "You shmell like a woshe *hic*... (so hot)" (rose -> roshe -> woshe)
        assert!(result.contains("shmell"));
        assert!(result.contains("woshe"));
        assert!(result.contains("*hic*"));
    }
}
