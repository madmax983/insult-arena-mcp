//! Single-player mode against an AI Sensei.
//!
//! The Dojo allows players to practice their insults against a simulated opponent
//! ([`Sensei`]) of varying difficulty levels.
//!
//! # Mechanics
//!
//! - **Single Player**: The player always acts as the Challenger initially.
//! - **Auto-Play**: The Sensei automatically responds to insults and attacks when it's their turn.
//! - **Hype Tracking**: The [`Audience`] tracks the excitement level of the match.

use crate::arena::{ArenaError, MAX_INPUT_LENGTH};
use crate::experimental::audience::{Audience, Reaction};
use crate::experimental::sensei::Sensei;
use crate::{Duel, DuelResult, DuelState, Duelist, ExchangeResult, InsultError};
use serde::{Deserialize, Serialize};

/// Events that occur during a Dojo turn.
///
/// These events describe the narrative flow of the match as the AI and player trade insults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DojoEvent {
    /// Result of the player's action (e.g., "You threw...", "Touché!").
    PlayerAction {
        /// Description of what happened.
        description: String,
        /// The new state of the duel.
        state: String,
    },
    /// The Sensei (AI) made a move (e.g., "Sensei parries...", "Sensei throws...").
    SenseiMove {
        /// The type of action ("parry", "fail", "attack").
        action: String,
        /// Description of what happened.
        description: String,
    },
    /// The audience reacted to an exchange (e.g., Cheer, Boo).
    AudienceReaction(Reaction),
    /// The duel has finished.
    GameOver(DuelResult),
}

/// A single-player training ground against an AI Sensei.
///
/// # Hero's Journey
///
/// ```
/// use insult_arena_mcp::experimental::dojo::{Dojo, DojoEvent};
///
/// // 1. Enter the Dojo (Difficulty 0.5)
/// let mut dojo = Dojo::new(0.5);
///
/// // 2. Throw an insult to start
/// let events = dojo.turn("You fight like a dairy farmer!").unwrap();
///
/// // 3. Process what happened
/// for event in events {
///     match event {
///         DojoEvent::PlayerAction { description, .. } => {
///             println!("You: {}", description);
///         }
///         DojoEvent::SenseiMove { description, .. } => {
///             println!("Sensei: {}", description);
///         }
///         DojoEvent::AudienceReaction(reaction) => {
///             println!("Crowd: {:?}", reaction);
///         }
///         DojoEvent::GameOver(result) => {
///             println!("Game Over! Winner: {}", result.winner);
///         }
///     }
/// }
/// ```
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
    /// Returns error if:
    /// - The input is too long (> [`MAX_INPUT_LENGTH`]).
    /// - The move is invalid for the current state (e.g., throwing insult when waiting for comeback).
    /// - The insult is unknown.
    ///
    /// # Panics
    ///
    /// This function may panic if the internal game state becomes inconsistent with the Sensei's logic
    /// (e.g., if the Sensei tries to respond to a non-existent insult). This should not happen
    /// under normal gameplay conditions.
    #[allow(clippy::expect_used)]
    pub fn turn(&mut self, input: &str) -> Result<Vec<DojoEvent>, ArenaError> {
        if input.len() > MAX_INPUT_LENGTH {
            return Err(ArenaError::InputTooLong(MAX_INPUT_LENGTH));
        }

        let mut events = Vec::new();

        self.process_player_move(input, &mut events)?;
        self.process_sensei_loop(&mut events);

        Ok(events)
    }

    fn process_player_move(
        &mut self,
        input: &str,
        events: &mut Vec<DojoEvent>,
    ) -> Result<(), ArenaError> {
        match self.duel.state() {
            DuelState::AwaitingInsult { attacker } => {
                if attacker == Duelist::Defender {
                    return Err(InsultError::WaitingForInsult.into());
                }
                self.duel.throw_insult(input.to_string())?;
                events.push(DojoEvent::PlayerAction {
                    description: format!("You threw: \"{input}\""),
                    state: "awaiting_comeback".to_string(),
                });
            }
            DuelState::AwaitingComeback { attacker } => {
                if attacker == Duelist::Challenger {
                    return Err(InsultError::WaitingForComeback.into());
                }
                let exchange = self.duel.respond(input.to_string())?;
                let reaction = self.audience.react(&exchange, None);
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
            DuelState::Finished { .. } => return Err(InsultError::DuelOver.into()),
        }
        Ok(())
    }

    #[allow(clippy::expect_used)] // Internal loop, panic on logic error
    fn process_sensei_loop(&mut self, events: &mut Vec<DojoEvent>) {
        loop {
            if self.duel.is_finished() {
                if let Some(result) = self.duel.result() {
                    events.push(DojoEvent::GameOver(result));
                }
                break;
            }

            match self.duel.state() {
                DuelState::AwaitingComeback { attacker } => {
                    if attacker == Duelist::Challenger {
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

                        events.push(DojoEvent::AudienceReaction(self.audience.react(&exchange, None)));
                    } else {
                        break;
                    }
                }
                DuelState::AwaitingInsult { attacker } => {
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
