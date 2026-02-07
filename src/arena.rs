//! Arena module: Encapsulates the game state and logic.

use crate::duel::{Duel, DuelState, Duelist, InsultError};
use serde::{Deserialize, Serialize};

pub const MAX_INPUT_LENGTH: usize = 1024;
pub const MAX_SESSION_ID_LENGTH: usize = 128;

/// Errors that can occur in the Arena.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ArenaError {
    #[error("No duel in progress. Call start_duel first!")]
    NoDuel,
    #[error("Input too long (max {0} chars)")]
    InputTooLong(usize),
    #[error("Session ID too long (max {0} chars)")]
    SessionIdTooLong(usize),
    #[error("{0} role is already taken!")]
    RoleTaken(String),
    #[error("It is not your turn! Waiting for {0}.")]
    NotYourTurn(String),
    #[error("Unknown insult: \"{0}\". Use list_insults to see valid options.")]
    UnknownInsult(String),
    #[error("No pending insult to hint about.")]
    NoPendingInsult,
    #[error("Could not find comeback for this insult.")]
    ComebackNotFound,
    #[error(transparent)]
    DuelError(#[from] InsultError),
}

/// The outcome of an action in the Arena.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArenaOutcome {
    DuelStarted,
    RoleRegistered { role: Duelist },
    InsultThrown { insult: String },
    ExchangeProcessed { exchange: crate::duel::Exchange },
}

/// Tracks which session is playing which role.
#[derive(Debug, Default)]
struct DuelSessions {
    challenger: Option<String>,
    defender: Option<String>,
}

/// Serializable view of the duel state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuelStateView {
    /// Current phase of the duel.
    pub phase: String,
    /// Who should act next (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_to_act: Option<String>,
    /// The pending insult waiting for a comeback.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_insult: Option<String>,
    /// Challenger's score.
    pub challenger_score: u8,
    /// Defender's score.
    pub defender_score: u8,
    /// Wins needed to win the duel.
    pub wins_needed: u8,
    /// The winner (if duel is over).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winner: Option<String>,
}

pub fn duel_state_view(duel: &Duel) -> DuelStateView {
    let (phase, next_to_act, winner) = match duel.state() {
        DuelState::AwaitingInsult { attacker } => (
            "awaiting_insult".to_string(),
            Some(attacker.to_string()),
            None,
        ),
        DuelState::AwaitingComeback { attacker } => (
            "awaiting_comeback".to_string(),
            Some(attacker.opponent().to_string()),
            None,
        ),
        DuelState::Finished { winner } => ("finished".to_string(), None, Some(winner.to_string())),
    };

    let (challenger_score, defender_score) = duel.scores();

    DuelStateView {
        phase,
        next_to_act,
        pending_insult: duel.pending_insult().map(String::from),
        challenger_score,
        defender_score,
        wins_needed: 3,
        winner,
    }
}

/// The Arena encapsulates the game state (Duel) and session management.
pub struct Arena {
    duel: Option<Duel>,
    sessions: DuelSessions,
}

impl Arena {
    #[must_use]
    pub fn new() -> Self {
        Self {
            duel: None,
            sessions: DuelSessions::default(),
        }
    }

    pub fn start_duel(&mut self) -> (ArenaOutcome, DuelStateView) {
        let duel = Duel::new();
        let view = duel_state_view(&duel);
        self.duel = Some(duel);

        // Clear session registrations for new duel
        self.sessions = DuelSessions::default();

        (ArenaOutcome::DuelStarted, view)
    }

    /// Register a session as the challenger.
    ///
    /// # Errors
    /// Returns error if the role is already taken.
    pub fn register_challenger(
        &mut self,
        session_id: String,
    ) -> Result<(ArenaOutcome, Option<DuelStateView>), ArenaError> {
        if session_id.len() > MAX_SESSION_ID_LENGTH {
            return Err(ArenaError::SessionIdTooLong(MAX_SESSION_ID_LENGTH));
        }

        if self.sessions.challenger.is_some() {
            return Err(ArenaError::RoleTaken("Challenger".to_string()));
        }

        self.sessions.challenger = Some(session_id);
        let state = self.duel.as_ref().map(duel_state_view);

        Ok((
            ArenaOutcome::RoleRegistered {
                role: Duelist::Challenger,
            },
            state,
        ))
    }

    /// Register a session as the defender.
    ///
    /// # Errors
    /// Returns error if the role is already taken.
    pub fn register_defender(
        &mut self,
        session_id: String,
    ) -> Result<(ArenaOutcome, Option<DuelStateView>), ArenaError> {
        if session_id.len() > MAX_SESSION_ID_LENGTH {
            return Err(ArenaError::SessionIdTooLong(MAX_SESSION_ID_LENGTH));
        }

        if self.sessions.defender.is_some() {
            return Err(ArenaError::RoleTaken("Defender".to_string()));
        }

        self.sessions.defender = Some(session_id);
        let state = self.duel.as_ref().map(duel_state_view);

        Ok((
            ArenaOutcome::RoleRegistered {
                role: Duelist::Defender,
            },
            state,
        ))
    }

    #[must_use]
    pub fn get_role_for_session(&self, session_id: &str) -> Option<Duelist> {
        if self.sessions.challenger.as_deref() == Some(session_id) {
            Some(Duelist::Challenger)
        } else if self.sessions.defender.as_deref() == Some(session_id) {
            Some(Duelist::Defender)
        } else {
            None
        }
    }

    /// Get the current state of the duel.
    ///
    /// # Errors
    /// Returns error if no duel is in progress.
    pub fn get_duel_state(
        &self,
        session_id: Option<&str>,
    ) -> Result<(DuelStateView, Option<String>), ArenaError> {
        let Some(duel) = self.duel.as_ref() else {
            return Err(ArenaError::NoDuel);
        };

        let view = duel_state_view(duel);
        let role = session_id.and_then(|id| self.get_role_for_session(id));

        Ok((view, role.map(|r| r.to_string())))
    }

    /// List all available insults.
    ///
    /// # Errors
    /// Returns error if no duel is in progress.
    pub fn list_insults(&self) -> Result<Vec<&str>, ArenaError> {
        let Some(duel) = self.duel.as_ref() else {
            return Err(ArenaError::NoDuel);
        };

        Ok(duel
            .insult_bank()
            .all_pairs()
            .iter()
            .map(|p| p.insult)
            .collect())
    }

    /// Throw an insult.
    ///
    /// # Errors
    /// Returns error if input is too long, no duel is in progress, or the insult is invalid/unexpected.
    pub fn throw_insult(
        &mut self,
        session_id: &str,
        insult: String,
    ) -> Result<(ArenaOutcome, DuelStateView), ArenaError> {
        if insult.len() > MAX_INPUT_LENGTH {
            return Err(ArenaError::InputTooLong(MAX_INPUT_LENGTH));
        }

        let Some(duel) = self.duel.as_mut() else {
            return Err(ArenaError::NoDuel);
        };

        // Validate turn/role
        if let DuelState::AwaitingInsult { attacker } = duel.state() {
            let expected_session = match attacker {
                Duelist::Challenger => self.sessions.challenger.as_ref(),
                Duelist::Defender => self.sessions.defender.as_ref(),
            };

            if expected_session.is_some_and(|expected| expected != session_id) {
                return Err(ArenaError::NotYourTurn(attacker.to_string()));
            }
        }

        // ⚡ Bolt Optimization: Avoid double allocation.
        // We need 'insult' for both Duel (consumed) and ArenaOutcome (consumed).
        // Clone once instead of creating two new Strings from &str.
        let insult_clone = insult.clone();

        match duel.throw_insult(insult) {
            Ok(()) => {
                let view = duel_state_view(duel);
                Ok((
                    ArenaOutcome::InsultThrown {
                        insult: insult_clone,
                    },
                    view,
                ))
            }
            // If it failed, we return the clone in the error
            Err(InsultError::UnknownInsult(_)) => Err(ArenaError::UnknownInsult(insult_clone)),
            Err(e) => Err(ArenaError::from(e)),
        }
    }

    /// Respond to an insult with a comeback.
    ///
    /// # Errors
    /// Returns error if input is too long, no duel is in progress, or it's not the comeback phase.
    pub fn respond(
        &mut self,
        session_id: &str,
        comeback: String,
    ) -> Result<(ArenaOutcome, DuelStateView), ArenaError> {
        if comeback.len() > MAX_INPUT_LENGTH {
            return Err(ArenaError::InputTooLong(MAX_INPUT_LENGTH));
        }

        let Some(duel) = self.duel.as_mut() else {
            return Err(ArenaError::NoDuel);
        };

        // Validate turn/role
        if let DuelState::AwaitingComeback { attacker } = duel.state() {
            let defender = attacker.opponent();
            let expected_session = match defender {
                Duelist::Challenger => self.sessions.challenger.as_ref(),
                Duelist::Defender => self.sessions.defender.as_ref(),
            };

            if expected_session.is_some_and(|expected| expected != session_id) {
                return Err(ArenaError::NotYourTurn(defender.to_string()));
            }
        }

        // ⚡ Bolt Optimization: Move 'comeback' directly to Duel.
        // Zero allocations here (was 1 from &str).
        match duel.respond(comeback) {
            Ok(exchange) => {
                let view = duel_state_view(duel);
                let outcome = ArenaOutcome::ExchangeProcessed { exchange };
                Ok((outcome, view))
            }
            Err(e) => Err(ArenaError::from(e)),
        }
    }

    /// Get a hint for the current pending insult.
    ///
    /// # Errors
    /// Returns error if no duel is in progress or no insult is pending.
    pub fn get_hint(&self) -> Result<(String, String), ArenaError> {
        let Some(duel) = self.duel.as_ref() else {
            return Err(ArenaError::NoDuel);
        };

        let Some(insult) = duel.pending_insult() else {
            return Err(ArenaError::NoPendingInsult);
        };

        let Some(comeback) = duel.insult_bank().find_comeback(insult) else {
            return Err(ArenaError::ComebackNotFound);
        };

        // Give a masked hint (Hangman style)
        let hint = crate::InsultBank::get_hint_masked(comeback);
        Ok((hint, insult.to_string()))
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn start_duel_creates_new_game() {
        let mut arena = Arena::new();
        let (outcome, view) = arena.start_duel();
        assert!(matches!(outcome, ArenaOutcome::DuelStarted));
        assert_eq!(view.phase, "awaiting_insult");
    }

    #[test]
    fn get_state_without_duel_returns_error() {
        let arena = Arena::new();
        let result = arena.get_duel_state(None);
        assert_eq!(result.unwrap_err(), ArenaError::NoDuel);
    }

    #[test]
    fn list_insults_returns_all_insults() {
        let mut arena = Arena::new();
        arena.start_duel();
        let insults = arena.list_insults().unwrap();
        assert!(insults.iter().any(|&s| s.contains("dairy farmer")));
    }

    #[test]
    fn full_exchange_flow() {
        let mut arena = Arena::new();
        arena.start_duel();

        // Throw insult
        let (outcome, view) = arena
            .throw_insult("p1", "You fight like a dairy farmer!".to_string())
            .unwrap();
        assert!(matches!(outcome, ArenaOutcome::InsultThrown { .. }));
        assert_eq!(view.phase, "awaiting_comeback");

        // Correct comeback
        let (outcome, view) = arena
            .respond("p2", "How appropriate. You fight like a cow!".to_string())
            .unwrap();
        assert!(matches!(outcome, ArenaOutcome::ExchangeProcessed { .. }));
        assert_eq!(view.phase, "awaiting_insult"); // Defender attacks next
    }

    #[test]
    fn register_roles() {
        let mut arena = Arena::new();

        let (outcome, _) = arena.register_challenger("session1".to_string()).unwrap();
        assert!(matches!(
            outcome,
            ArenaOutcome::RoleRegistered {
                role: Duelist::Challenger
            }
        ));

        let (outcome, _) = arena.register_defender("session2".to_string()).unwrap();
        assert!(matches!(
            outcome,
            ArenaOutcome::RoleRegistered {
                role: Duelist::Defender
            }
        ));

        // Can't register twice
        let result = arena.register_challenger("session3".to_string());
        assert!(matches!(result, Err(ArenaError::RoleTaken(_))));
    }

    #[test]
    fn rejects_excessive_input_length() {
        let mut arena = Arena::new();
        arena.start_duel();

        let long_string = "a".repeat(5000);
        let result = arena.throw_insult("p1", long_string.clone());

        assert!(matches!(result, Err(ArenaError::InputTooLong(_))));

        let result = arena.respond("p1", long_string);
        assert!(matches!(result, Err(ArenaError::InputTooLong(_))));
    }

    #[test]
    fn rejects_excessive_session_id_length() {
        let mut arena = Arena::new();
        let long_id = "s".repeat(200);

        let result = arena.register_challenger(long_id.clone());
        assert!(matches!(result, Err(ArenaError::SessionIdTooLong(_))));

        let result = arena.register_defender(long_id);
        assert!(matches!(result, Err(ArenaError::SessionIdTooLong(_))));
    }

    #[test]
    fn beggar_manners_insult_exchange_works() {
        let mut arena = Arena::new();
        arena.start_duel();

        // Throw "beggar manners" insult
        let (_, view) = arena
            .throw_insult("p1", "You have the manners of a beggar.".to_string())
            .unwrap();
        assert_eq!(view.phase, "awaiting_comeback");

        // Respond with correct comeback
        let (_, view) = arena
            .respond(
                "p2",
                "I wanted to make sure you'd feel comfortable with me.".to_string(),
            )
            .unwrap();
        assert_eq!(view.phase, "awaiting_insult"); // Defender attacks next
    }

    #[test]
    fn throw_insult_without_duel_returns_error() {
        let mut arena = Arena::new();
        let result = arena.throw_insult("p1", "foo".to_string());
        assert_eq!(result.unwrap_err(), ArenaError::NoDuel);
    }

    #[test]
    fn respond_without_duel_returns_error() {
        let mut arena = Arena::new();
        let result = arena.respond("p1", "bar".to_string());
        assert_eq!(result.unwrap_err(), ArenaError::NoDuel);
    }

    #[test]
    fn state_mismatch_errors_are_reported() {
        let mut arena = Arena::new();
        arena.start_duel();

        // 1. Throw insult -> OK
        arena
            .throw_insult("p1", "You fight like a dairy farmer!".to_string())
            .unwrap();

        // 2. Throw insult AGAIN -> Error (Waiting for comeback)
        let result = arena.throw_insult("p1", "You fight like a dairy farmer!".to_string());
        assert!(matches!(
            result,
            Err(ArenaError::DuelError(InsultError::WaitingForComeback))
        ));

        // 3. Respond -> OK (Parried, Defender becomes attacker)
        arena
            .respond("p2", "How appropriate. You fight like a cow!".to_string())
            .unwrap();

        // 4. Respond AGAIN -> Error (Waiting for insult)
        let result = arena.respond("p2", "Too late".to_string());
        assert!(matches!(
            result,
            Err(ArenaError::DuelError(InsultError::WaitingForInsult))
        ));
    }

    #[test]
    fn actions_after_duel_finished_return_error() {
        let mut arena = Arena::new();
        arena.start_duel();

        // Win the duel (Challenger wins 3 times)
        for _ in 0..3 {
            arena
                .throw_insult("p1", "You fight like a dairy farmer!".to_string())
                .unwrap();
            arena.respond("p2", "wrong".to_string()).unwrap();
        }

        // Duel should be finished
        let (view, _) = arena.get_duel_state(None).unwrap();
        assert_eq!(view.phase, "finished");

        // Throw insult -> Error
        let result = arena.throw_insult("p1", "You fight like a dairy farmer!".to_string());
        assert!(matches!(
            result,
            Err(ArenaError::DuelError(InsultError::DuelOver))
        ));

        // Respond -> Error
        let result = arena.respond("p2", "wrong".to_string());
        assert!(matches!(
            result,
            Err(ArenaError::DuelError(InsultError::DuelOver))
        ));
    }

    #[test]
    fn test_turn_enforcement_and_role_protection() {
        let mut arena = Arena::new();
        arena.start_duel();

        // Register roles
        arena.register_challenger("alice".to_string()).unwrap();
        arena.register_defender("bob".to_string()).unwrap();

        // 1. Intruder cannot throw insult
        let result = arena.throw_insult("eve", "You fight like a dairy farmer!".to_string());
        assert!(matches!(result, Err(ArenaError::NotYourTurn(_))));

        // 2. Defender cannot throw insult (it's Challenger's turn)
        let result = arena.throw_insult("bob", "You fight like a dairy farmer!".to_string());
        assert!(matches!(result, Err(ArenaError::NotYourTurn(_))));

        // 3. Challenger CAN throw insult
        let (_, view) = arena
            .throw_insult("alice", "You fight like a dairy farmer!".to_string())
            .unwrap();
        assert_eq!(view.phase, "awaiting_comeback");

        // 4. Intruder cannot respond
        let result = arena.respond("eve", "How appropriate. You fight like a cow!".to_string());
        assert!(matches!(result, Err(ArenaError::NotYourTurn(_))));

        // 5. Challenger cannot respond (it's Defender's turn)
        let result = arena.respond(
            "alice",
            "How appropriate. You fight like a cow!".to_string(),
        );
        assert!(matches!(result, Err(ArenaError::NotYourTurn(_))));

        // 6. Defender CAN respond
        let (_, view) = arena
            .respond("bob", "How appropriate. You fight like a cow!".to_string())
            .unwrap();
        assert_eq!(view.phase, "awaiting_insult");

        // Now Defender is attacker.

        // 7. Challenger cannot throw insult (now Defender's turn)
        let result = arena.throw_insult("alice", "You fight like a dairy farmer!".to_string());
        assert!(matches!(result, Err(ArenaError::NotYourTurn(_))));

        // 8. Defender CAN throw insult
        let (outcome, _) = arena
            .throw_insult("bob", "You fight like a dairy farmer!".to_string())
            .unwrap();
        assert!(matches!(outcome, ArenaOutcome::InsultThrown { .. }));
    }
}
