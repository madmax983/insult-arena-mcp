//! Arena module: Encapsulates the game state and logic.

use crate::duel::{Duel, DuelState, Duelist, ExchangeResult, InsultError};
use serde::{Deserialize, Serialize};

pub const MAX_INPUT_LENGTH: usize = 1024;

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

/// Response from the arena/server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuelResponse {
    /// Whether the action succeeded.
    pub success: bool,
    /// Human-readable message about what happened.
    pub message: String,
    /// Current state of the duel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<DuelStateView>,
    /// Your role in this duel (if registered).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub your_role: Option<String>,
}

impl DuelResponse {
    pub fn success(message: impl Into<String>, state: DuelStateView) -> Self {
        Self {
            success: true,
            message: message.into(),
            state: Some(state),
            your_role: None,
        }
    }

    pub fn success_with_role(message: impl Into<String>, state: DuelStateView, role: &str) -> Self {
        Self {
            success: true,
            message: message.into(),
            state: Some(state),
            your_role: Some(role.to_string()),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            state: None,
            your_role: None,
        }
    }

    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self)
            .unwrap_or_else(|_| "Error serializing response".to_string())
    }
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

    pub fn start_duel(&mut self) -> (String, DuelStateView) {
        let duel = Duel::new();
        let view = duel_state_view(&duel);
        self.duel = Some(duel);

        // Clear session registrations for new duel
        self.sessions = DuelSessions::default();

        (
            "En garde! A new duel begins. Challenger throws the first insult!".to_string(),
            view,
        )
    }

    /// Register a session as the challenger.
    ///
    /// # Errors
    /// Returns error if the role is already taken.
    pub fn register_challenger(
        &mut self,
        session_id: String,
    ) -> Result<(String, Option<DuelStateView>), String> {
        if self.sessions.challenger.is_some() {
            return Err("Challenger role is already taken!".to_string());
        }

        self.sessions.challenger = Some(session_id);
        let state = self.duel.as_ref().map(duel_state_view);

        Ok((
            "You are now the Challenger! Throw the first insult when ready.".to_string(),
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
    ) -> Result<(String, Option<DuelStateView>), String> {
        if self.sessions.defender.is_some() {
            return Err("Defender role is already taken!".to_string());
        }

        self.sessions.defender = Some(session_id);
        let state = self.duel.as_ref().map(duel_state_view);

        Ok((
            "You are now the Defender! Wait for an insult, then respond with a comeback."
                .to_string(),
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
    ) -> Result<(DuelStateView, Option<String>), String> {
        let Some(duel) = self.duel.as_ref() else {
            return Err("No duel in progress. Call start_duel first!".to_string());
        };

        let view = duel_state_view(duel);
        let role = session_id.and_then(|id| self.get_role_for_session(id));

        Ok((view, role.map(|r| r.to_string())))
    }

    /// List all available insults.
    ///
    /// # Errors
    /// Returns error if no duel is in progress.
    pub fn list_insults(&self) -> Result<Vec<&str>, String> {
        let Some(duel) = self.duel.as_ref() else {
            return Err("No duel in progress. Call start_duel first!".to_string());
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
    pub fn throw_insult(&mut self, insult: &str) -> Result<(String, DuelStateView), String> {
        if insult.len() > MAX_INPUT_LENGTH {
            return Err("Input too long".to_string());
        }

        let Some(duel) = self.duel.as_mut() else {
            return Err("No duel in progress. Call start_duel first!".to_string());
        };

        match duel.throw_insult(insult.to_string()) {
            Ok(()) => {
                let view = duel_state_view(duel);
                Ok((
                    format!("You hurl the insult: \"{insult}\" - awaiting comeback!"),
                    view,
                ))
            }
            Err(InsultError::UnknownInsult(insult)) => Err(format!(
                "Unknown insult: \"{insult}\". Use list_insults to see valid options."
            )),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Respond to an insult with a comeback.
    ///
    /// # Errors
    /// Returns error if input is too long, no duel is in progress, or it's not the comeback phase.
    pub fn respond(&mut self, comeback: &str) -> Result<(String, DuelStateView), String> {
        if comeback.len() > MAX_INPUT_LENGTH {
            return Err("Input too long".to_string());
        }

        let Some(duel) = self.duel.as_mut() else {
            return Err("No duel in progress. Call start_duel first!".to_string());
        };

        match duel.respond(comeback.to_string()) {
            Ok(exchange) => {
                let view = duel_state_view(duel);
                let is_finished = duel.is_finished();

                let message = if exchange.result.is_parried() {
                    if is_finished {
                        format!("TOUCHÉ! Perfect parry! {} wins the duel!", exchange.winner)
                    } else {
                        format!(
                            "TOUCHÉ! Perfect parry! {} wins the exchange and attacks next!",
                            exchange.winner
                        )
                    }
                } else {
                    let expected =
                        if let ExchangeResult::Failed { ref correct, .. } = exchange.result {
                            correct.clone()
                        } else {
                            String::new()
                        };

                    if is_finished {
                        format!(
                            "You failed to parry! {} wins the duel!\n\nExpected comeback: \"{}\"",
                            exchange.winner, expected
                        )
                    } else {
                        format!(
                            "You failed to parry! {} wins the exchange and attacks again!\n\nExpected comeback: \"{}\"",
                            exchange.winner, expected
                        )
                    }
                };

                Ok((message, view))
            }
            Err(e) => Err(e.to_string()),
        }
    }

    /// Get a hint for the current pending insult.
    ///
    /// # Errors
    /// Returns error if no duel is in progress or no insult is pending.
    pub fn get_hint(&self) -> Result<(String, String), String> {
        let Some(duel) = self.duel.as_ref() else {
            return Err("No duel in progress. Call start_duel first!".to_string());
        };

        let Some(insult) = duel.pending_insult() else {
            return Err("No pending insult to hint about.".to_string());
        };

        let Some(comeback) = duel.insult_bank().find_comeback(insult) else {
            return Err("Could not find comeback for this insult.".to_string());
        };

        // Give first 20 characters as hint
        let hint: String = comeback.chars().take(20).collect();
        Ok((format!("{hint}..."), insult.to_string()))
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
        let (msg, view) = arena.start_duel();
        assert!(msg.contains("En garde"));
        assert_eq!(view.phase, "awaiting_insult");
    }

    #[test]
    fn get_state_without_duel_returns_error() {
        let arena = Arena::new();
        let result = arena.get_duel_state(None);
        assert!(result.is_err());
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
        let (msg, view) = arena
            .throw_insult("You fight like a dairy farmer!")
            .unwrap();
        assert!(msg.contains("awaiting comeback"));
        assert_eq!(view.phase, "awaiting_comeback");

        // Correct comeback
        let (msg, view) = arena
            .respond("How appropriate. You fight like a cow!")
            .unwrap();
        assert!(msg.contains("TOUCHÉ"));
        assert_eq!(view.phase, "awaiting_insult"); // Defender attacks next
    }

    #[test]
    fn register_roles() {
        let mut arena = Arena::new();

        let (msg, _) = arena.register_challenger("session1".to_string()).unwrap();
        assert!(msg.contains("Challenger"));

        let (msg, _) = arena.register_defender("session2".to_string()).unwrap();
        assert!(msg.contains("Defender"));

        // Can't register twice
        let result = arena.register_challenger("session3".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn rejects_excessive_input_length() {
        let mut arena = Arena::new();
        arena.start_duel();

        let long_string = "a".repeat(5000);
        let result = arena.throw_insult(&long_string);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Input too long");

        let result = arena.respond(&long_string);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Input too long");
    }

    #[test]
    fn beggar_manners_insult_exchange_works() {
        let mut arena = Arena::new();
        arena.start_duel();

        // Throw "beggar manners" insult
        let (msg, view) = arena
            .throw_insult("You have the manners of a beggar.")
            .unwrap();
        assert!(
            msg.contains("awaiting comeback"),
            "Should be waiting for comeback"
        );
        assert_eq!(view.phase, "awaiting_comeback");

        // Respond with correct comeback
        let (msg, view) = arena
            .respond("I wanted to make sure you'd feel comfortable with me.")
            .unwrap();
        assert!(msg.contains("TOUCHÉ"), "Should parry successfully");
        assert_eq!(view.phase, "awaiting_insult"); // Defender attacks next
    }

    #[test]
    fn throw_insult_without_duel_returns_error() {
        let mut arena = Arena::new();
        let result = arena.throw_insult("foo");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No duel in progress"));
    }

    #[test]
    fn respond_without_duel_returns_error() {
        let mut arena = Arena::new();
        let result = arena.respond("bar");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No duel in progress"));
    }

    #[test]
    fn state_mismatch_errors_are_reported() {
        let mut arena = Arena::new();
        arena.start_duel();

        // 1. Throw insult -> OK
        arena
            .throw_insult("You fight like a dairy farmer!")
            .unwrap();

        // 2. Throw insult AGAIN -> Error (Waiting for comeback)
        let result = arena.throw_insult("You fight like a dairy farmer!");
        assert!(result.is_err());
        assert!(
            result.unwrap_err().contains("Waiting for a comeback"),
            "Should error when throwing insult while awaiting comeback"
        );

        // 3. Respond -> OK (Parried, Defender becomes attacker)
        arena
            .respond("How appropriate. You fight like a cow!")
            .unwrap();

        // 4. Respond AGAIN -> Error (Waiting for insult)
        let result = arena.respond("Too late");
        assert!(result.is_err());
        assert!(
            result.unwrap_err().contains("Waiting for an insult"),
            "Should error when responding while awaiting insult"
        );
    }

    #[test]
    fn actions_after_duel_finished_return_error() {
        let mut arena = Arena::new();
        arena.start_duel();

        // Win the duel (Challenger wins 3 times)
        for _ in 0..3 {
            arena
                .throw_insult("You fight like a dairy farmer!")
                .unwrap();
            arena.respond("wrong").unwrap();
        }

        // Duel should be finished
        let (view, _) = arena.get_duel_state(None).unwrap();
        assert_eq!(view.phase, "finished");

        // Throw insult -> Error
        let result = arena.throw_insult("You fight like a dairy farmer!");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("duel is over"));

        // Respond -> Error
        let result = arena.respond("wrong");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("duel is over"));
    }
}
