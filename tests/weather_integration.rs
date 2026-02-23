//! Integration tests for the Weather System.
//!
//! Verifies that weather conditions correctly modify game mechanics,
//! specifically audience hype reactions.

#![cfg(feature = "nova")]

use insult_arena_mcp::Duelist;
use insult_arena_mcp::Exchange;
use insult_arena_mcp::ExchangeResult;
use insult_arena_mcp::experimental::audience::Audience;
use insult_arena_mcp::experimental::weather::{WeatherCondition, WeatherSystem};

#[test]
fn storm_amplifies_hype() {
    let mut audience = Audience::new();
    let initial_hype = audience.hype; // 50

    let mut weather = WeatherSystem::new();
    weather.current = WeatherCondition::Storm;

    let exchange = Exchange {
        attacker: Duelist::Challenger,
        result: ExchangeResult::Parried {
            insult: "Insult".into(),
            comeback: "Comeback".into(),
        },
        winner: Duelist::Defender,
    };

    // Parry normally gives +10.
    // Storm should amplify to +15 (10 * 1.5).
    audience.react(&exchange, Some(&weather));

    let expected_hype = initial_hype + 15;
    assert_eq!(
        audience.hype, expected_hype,
        "Storm should amplify hype gain to +15"
    );
}

#[test]
fn heatwave_dampens_hype() {
    let mut audience = Audience::new();
    let initial_hype = audience.hype; // 50

    let mut weather = WeatherSystem::new();
    weather.current = WeatherCondition::Heatwave;

    let exchange = Exchange {
        attacker: Duelist::Challenger,
        result: ExchangeResult::Parried {
            insult: "Insult".into(),
            comeback: "Comeback".into(),
        },
        winner: Duelist::Defender,
    };

    // Parry normally gives +10.
    // Heatwave should dampen to +5 (10 * 0.5).
    audience.react(&exchange, Some(&weather));

    let expected_hype = initial_hype + 5;
    assert_eq!(
        audience.hype, expected_hype,
        "Heatwave should dampen hype gain to +5"
    );
}

#[test]
fn clear_weather_no_effect() {
    let mut audience = Audience::new();
    let initial_hype = audience.hype; // 50

    let mut weather = WeatherSystem::new();
    weather.current = WeatherCondition::Clear;

    let exchange = Exchange {
        attacker: Duelist::Challenger,
        result: ExchangeResult::Parried {
            insult: "Insult".into(),
            comeback: "Comeback".into(),
        },
        winner: Duelist::Defender,
    };

    // Parry gives +10.
    audience.react(&exchange, Some(&weather));

    let expected_hype = initial_hype + 10;
    assert_eq!(
        audience.hype, expected_hype,
        "Clear weather should have standard hype gain (+10)"
    );
}
