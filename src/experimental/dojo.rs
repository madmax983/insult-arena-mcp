use crate::duel::Duel;
use crate::error::InsultError;
use crate::experimental::audience::{Audience, Reaction};
use crate::model::{DuelResult, DuelState, Duelist, ExchangeResult};
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Events that occur during a Dojo turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DojoEvent {
    /// Result of the player's action.
    PlayerAction { description: String, state: String },
    /// The Sensei (AI) made a move.
    SenseiMove { action: String, description: String },
    /// The audience reacted to an exchange.
    AudienceReaction(Reaction),
    /// The duel has finished.
    GameOver(DuelResult),
}

/// A single-player training ground against an AI Sensei.
pub struct Dojo {
    /// The underlying duel state.
    duel: Duel,
    /// The virtual audience tracking hype.
    audience: Audience,
    /// The skill level of the Sensei (0.0 to 1.0).
    /// Represents the probability of the Sensei finding the correct comeback.
    sensei_skill: f64,
}

impl Dojo {
    /// Creates a new Dojo session.
    ///
    /// # Arguments
    ///
    /// * `skill` - Difficulty level (0.0 = total noob, 1.0 = unbeatable master).
    #[must_use]
    pub const fn new(skill: f64) -> Self {
        let skill = if skill < 0.0 {
            0.0
        } else if skill > 1.0 {
            1.0
        } else {
            skill
        };

        Self {
            duel: Duel::new(),
            audience: Audience::new(),
            sensei_skill: skill,
        }
    }

    /// Process a player's input and auto-play the Sensei's turns until it's the player's turn again.
    ///
    /// # Errors
    ///
    /// Returns error if the move is invalid for the current state.
    pub fn turn(&mut self, input: &str) -> Result<Vec<DojoEvent>, InsultError> {
        let mut events = Vec::new();

        // 1. Process Player's Move
        match self.duel.state() {
            DuelState::AwaitingInsult { attacker } => {
                if attacker == Duelist::Defender {
                    return Err(InsultError::WaitingForInsult); // Should be Sensei's turn, but let's check
                }
                // Player (Challenger) throws insult
                self.duel.throw_insult(input.to_string())?;
                events.push(DojoEvent::PlayerAction {
                    description: format!("You threw: \"{input}\""),
                    state: "awaiting_comeback".to_string(),
                });
            }
            DuelState::AwaitingComeback { attacker } => {
                if attacker == Duelist::Challenger {
                    return Err(InsultError::WaitingForComeback);
                }
                // Player (Defender) responds
                let exchange = self.duel.respond(input.to_string())?;

                // Add Audience Reaction
                let reaction = self.audience.react(&exchange);
                events.push(DojoEvent::AudienceReaction(reaction));

                match exchange.result {
                    ExchangeResult::Parried { .. } => {
                        events.push(DojoEvent::PlayerAction {
                            description: "Touché! You parried successfully!".to_string(),
                            state: "awaiting_insult".to_string(),
                        });
                    }
                    ExchangeResult::Failed { correct, .. } => {
                        events.push(DojoEvent::PlayerAction {
                            description: format!("Failed! You should have said: \"{correct}\""),
                            state: "awaiting_insult".to_string(),
                        });
                    }
                }
            }
            DuelState::Finished { .. } => return Err(InsultError::DuelOver),
        }

        // 2. Sensei Loop
        // While it is the Sensei's turn (Defender) or Sensei is Attacker, keep going.
        // The Sensei is the "Defender" in the Duel struct if Player is "Challenger".
        // Wait, Player is ALWAYS Challenger in this impl?
        // Yes, `Duel::new()` starts with Challenger attacking.
        // So Sensei is implicitly the Defender role initially.

        loop {
            if self.duel.is_finished() {
                if let Some(result) = self.duel.result() {
                    events.push(DojoEvent::GameOver(result));
                }
                break;
            }

            match self.duel.state() {
                DuelState::AwaitingComeback { attacker } => {
                    // If Attacker is Challenger (Player), then it's Defender (Sensei)'s turn to respond.
                    if attacker == Duelist::Challenger {
                        // Sensei responds
                        let event = self.sensei_defend();

                        // Capture audience reaction from the exchange we just made
                        if let Some(last_exchange) = self.duel.exchanges().last() {
                            events.push(DojoEvent::AudienceReaction(
                                self.audience.react(last_exchange),
                            ));
                        }

                        events.push(event);
                    } else {
                        // Attacker is Defender (Sensei), so it's Challenger (Player)'s turn to respond.
                        // Break loop to let player act.
                        break;
                    }
                }
                DuelState::AwaitingInsult { attacker } => {
                    // If Attacker is Defender (Sensei), Sensei throws insult.
                    if attacker == Duelist::Defender {
                        let event = self.sensei_attack();
                        events.push(event);
                    } else {
                        // Attacker is Challenger (Player). Break loop to let player act.
                        break;
                    }
                }
                DuelState::Finished { .. } => {
                    if let Some(result) = self.duel.result() {
                        events.push(DojoEvent::GameOver(result));
                    }
                    break;
                }
            }
        }

        Ok(events)
    }

    #[allow(clippy::expect_used)]
    fn sensei_attack(&mut self) -> DojoEvent {
        let insult = self.duel.insult_bank().random_insult().insult.to_string();
        // Sensei always picks a valid insult
        self.duel
            .throw_insult(insult.clone())
            .expect("Sensei picked invalid insult"); // Should not happen

        DojoEvent::SenseiMove {
            action: "attack".to_string(),
            description: format!("Sensei throws: \"{insult}\""),
        }
    }

    #[allow(clippy::expect_used)]
    fn sensei_defend(&mut self) -> DojoEvent {
        let pending = self
            .duel
            .pending_insult()
            .expect("Should be pending insult")
            .to_string(); // Clone to satisfy borrow checker

        let should_succeed = rand::thread_rng().gen_bool(self.sensei_skill);

        let response = if should_succeed {
            // Find correct comeback
            self.duel
                .insult_bank()
                .find_comeback(&pending)
                .unwrap_or("...")
                .to_string()
        } else {
            // Fail intentionally
            "I am rubber, you are glue!".to_string()
        };

        let exchange = self
            .duel
            .respond(response.clone())
            .expect("Sensei response failed");

        if exchange.result.is_parried() {
            DojoEvent::SenseiMove {
                action: "parry".to_string(),
                description: format!("Sensei parries: \"{response}\""),
            }
        } else {
            DojoEvent::SenseiMove {
                action: "fail".to_string(),
                description: format!("Sensei trips up: \"{response}\""),
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn dojo_initialization() {
        let dojo = Dojo::new(0.5);
        assert!((dojo.sensei_skill - 0.5).abs() < f64::EPSILON);
        assert!(!dojo.duel.is_finished());
    }

    #[test]
    fn player_starts_exchange() {
        let mut dojo = Dojo::new(1.0); // Sensei always parries
        let events = dojo.turn("You fight like a dairy farmer!").unwrap();

        // Expected flow:
        // 1. Player throws insult.
        // 2. Sensei responds (Parries).
        // 3. Sensei wins exchange -> Sensei attacks.
        // 4. Loop breaks (waiting for Player comeback).

        // Let's check the event stream
        assert!(events.len() >= 3);

        // Event 0: Player Action
        match &events[0] {
            DojoEvent::PlayerAction { description, .. } => {
                assert!(description.contains("You threw"));
            }
            _ => panic!("Expected PlayerAction"),
        }

        // Event: Audience Reaction (to Sensei's parry)
        // Note: The order in logic:
        // Sensei Defend -> Call Respond -> Duel updates -> Audience React -> Event Pushed.
        // Wait, in my code I pushed AudienceReaction *after* `sensei_defend` call but using `last_exchange`.

        // Let's just check existence of events.
        assert!(
            events
                .iter()
                .any(|e| matches!(e, DojoEvent::SenseiMove { action, .. } if action == "parry"))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, DojoEvent::SenseiMove { action, .. } if action == "attack"))
        );
    }

    #[test]
    fn dumb_sensei_fails() {
        let mut dojo = Dojo::new(0.0); // Sensei always fails
        let events = dojo.turn("You fight like a dairy farmer!").unwrap();

        // Expected flow:
        // 1. Player throws insult.
        // 2. Sensei responds (Fails).
        // 3. Player wins exchange -> Player attacks.
        // 4. Loop breaks (waiting for Player insult).

        assert!(
            events
                .iter()
                .any(|e| matches!(e, DojoEvent::SenseiMove { action, .. } if action == "fail"))
        );

        // Should NOT see Sensei attack, because Player is attacker now
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, DojoEvent::SenseiMove { action, .. } if action == "attack"))
        );

        // Next turn should accept insult
        assert!(matches!(
            dojo.duel.state(),
            DuelState::AwaitingInsult {
                attacker: Duelist::Challenger
            }
        ));
    }
}
