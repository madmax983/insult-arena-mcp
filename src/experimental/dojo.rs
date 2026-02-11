use crate::experimental::audience::{Audience, Reaction};
use crate::experimental::sensei::Sensei;
use crate::{Duel, DuelResult, DuelState, Duelist, ExchangeResult, InsultError};
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
    /// The AI Sensei opponent.
    sensei: Sensei,
}

impl Dojo {
    /// Creates a new Dojo session.
    ///
    /// # Arguments
    ///
    /// * `skill` - Difficulty level (0.0 = total noob, 1.0 = unbeatable master).
    #[must_use]
    pub fn new(skill: f64) -> Self {
        Self {
            duel: Duel::new(),
            audience: Audience::new(),
            sensei: Sensei::new(skill),
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
                        let pending = self
                            .duel
                            .pending_insult()
                            .expect("Should be pending insult")
                            .to_string();

                        let response = self.sensei.defend(self.duel.insult_bank(), &pending);

                        let exchange = self
                            .duel
                            .respond(response.clone())
                            .expect("Sensei response failed");

                        let description = if exchange.result.is_parried() {
                            format!("Sensei parries: \"{response}\"")
                        } else {
                            format!("Sensei trips up: \"{response}\"")
                        };

                        let action = if exchange.result.is_parried() {
                            "parry"
                        } else {
                            "fail"
                        };

                        events.push(DojoEvent::SenseiMove {
                            action: action.to_string(),
                            description,
                        });

                        // Capture audience reaction
                        events.push(DojoEvent::AudienceReaction(
                            self.audience.react(&exchange),
                        ));
                    } else {
                        // Attacker is Defender (Sensei), so it's Challenger (Player)'s turn to respond.
                        break;
                    }
                }
                DuelState::AwaitingInsult { attacker } => {
                    // If Attacker is Defender (Sensei), Sensei throws insult.
                    if attacker == Duelist::Defender {
                        let insult = self.sensei.attack(self.duel.insult_bank());

                        self.duel
                            .throw_insult(insult.clone())
                            .expect("Sensei picked invalid insult");

                        events.push(DojoEvent::SenseiMove {
                            action: "attack".to_string(),
                            description: format!("Sensei throws: \"{insult}\""),
                        });
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
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn dojo_initialization() {
        let dojo = Dojo::new(0.5);
        // Can't check private field 'sensei' directly, but we can verify Dojo state
        assert!(!dojo.duel.is_finished());
    }

    #[test]
    fn player_starts_exchange() {
        let mut dojo = Dojo::new(1.0); // Sensei always parries
        let events = dojo.turn("You fight like a dairy farmer!").unwrap();

        assert!(events.len() >= 3);

        match &events[0] {
            DojoEvent::PlayerAction { description, .. } => {
                assert!(description.contains("You threw"));
            }
            _ => panic!("Expected PlayerAction"),
        }

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

        assert!(
            events
                .iter()
                .any(|e| matches!(e, DojoEvent::SenseiMove { action, .. } if action == "fail"))
        );

        assert!(
            !events
                .iter()
                .any(|e| matches!(e, DojoEvent::SenseiMove { action, .. } if action == "attack"))
        );

        assert!(matches!(
            dojo.duel.state(),
            DuelState::AwaitingInsult {
                attacker: Duelist::Challenger
            }
        ));
    }

    #[test]
    fn dojo_turn_loop_correctness() {
        let mut dojo = Dojo::new(1.0);
        let _events = dojo.turn("You fight like a dairy farmer!").unwrap();

        assert!(matches!(
            dojo.duel.state(),
            DuelState::AwaitingComeback {
                attacker: Duelist::Defender
            }
        ));

        let pending = dojo
            .duel
            .pending_insult()
            .expect("Sensei should have thrown insult");
        let correct_response = dojo
            .duel
            .insult_bank()
            .find_comeback(pending)
            .unwrap()
            .to_string();

        let events2 = dojo.turn(&correct_response).unwrap();

        assert!(matches!(
            dojo.duel.state(),
            DuelState::AwaitingInsult {
                attacker: Duelist::Challenger
            }
        ));

        assert!(events2.iter().any(|e| matches!(e, DojoEvent::PlayerAction { description, .. } if description.contains("Touché"))));
    }
}
