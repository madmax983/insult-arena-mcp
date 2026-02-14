//! Arena module: Encapsulates the game state and logic.
//!
//! # Hero's Journey
//!
//! ```
//! use insult_arena_mcp::{Arena, SessionId, PlayerInput};
//!
//! // 1. Create the Arena
//! let mut arena = Arena::new();
//!
//! // 2. Start a new duel
//! let (outcome, view) = arena.start_duel().unwrap();
//! assert_eq!(view.phase, "awaiting_insult");
//!
//! // 3. Register players
//! let session_a = SessionId::new("session_A").unwrap();
//! let session_b = SessionId::new("session_B").unwrap();
//! arena.register_challenger(session_a.clone()).unwrap();
//! arena.register_defender(session_b.clone()).unwrap();
//!
//! // 4. Challenger throws an insult
//! let insult = PlayerInput::new("You fight like a dairy farmer!").unwrap();
//! let (outcome, view) = arena.throw_insult(&session_a, insult).unwrap();
//!
//! // 5. Defender responds
//! let comeback = PlayerInput::new("How appropriate. You fight like a cow!").unwrap();
//! let (outcome, view) = arena.respond(&session_b, comeback).unwrap();
//!
//! // Defender won the exchange!
//! assert_eq!(view.defender_score, 1);
//! ```

use crate::duel::{Duel, DuelState, DuelStateView, Duelist, InsultCheckError, InsultError};

/// Maximum length of any user input string (insults, comebacks).
///
/// This limit prevents Denial of Service (`DoS`) attacks via memory exhaustion.
pub const MAX_INPUT_LENGTH: usize = 1024;

/// Maximum length of a session ID string.
///
/// This limit prevents Denial of Service (`DoS`) attacks where malicious clients
/// send excessively long session IDs.
pub const MAX_SESSION_ID_LENGTH: usize = 128;

/// A validated session ID string.
///
/// Enforces `MAX_SESSION_ID_LENGTH`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(String);

impl SessionId {
    /// Creates a new `SessionId` from a string if it meets length requirements.
    ///
    /// # Errors
    /// Returns `ArenaError::SessionIdTooLong` if the string exceeds `MAX_SESSION_ID_LENGTH`.
    pub fn new(id: impl Into<String>) -> Result<Self, ArenaError> {
        let id = id.into();
        if id.len() > MAX_SESSION_ID_LENGTH {
            Err(ArenaError::SessionIdTooLong(MAX_SESSION_ID_LENGTH))
        } else {
            Ok(Self(id))
        }
    }

    /// Returns the inner string.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }

    /// Returns a string slice of the ID.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for SessionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for SessionId {
    type Error = ArenaError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// A validated player input string (insult or comeback).
///
/// Enforces `MAX_INPUT_LENGTH`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerInput(String);

impl PlayerInput {
    /// Creates a new `PlayerInput` from a string if it meets length requirements.
    ///
    /// # Errors
    /// Returns `ArenaError::InputTooLong` if the string exceeds `MAX_INPUT_LENGTH`.
    pub fn new(input: impl Into<String>) -> Result<Self, ArenaError> {
        let input = input.into();
        if input.len() > MAX_INPUT_LENGTH {
            Err(ArenaError::InputTooLong(MAX_INPUT_LENGTH))
        } else {
            Ok(Self(input))
        }
    }

    /// Returns the inner string.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }

    /// Returns a string slice of the input.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for PlayerInput {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PlayerInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for PlayerInput {
    type Error = ArenaError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

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
    #[error("A duel is already in progress. Wait for it to finish!")]
    DuelInProgress,
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
///
/// This internal struct maps the abstract roles ([`Duelist::Challenger`] and [`Duelist::Defender`])
/// to concrete session IDs provided by the MCP runtime.
#[derive(Debug, Default)]
struct DuelSessions {
    challenger: Option<SessionId>,
    defender: Option<SessionId>,
}

impl DuelSessions {
    fn register(&mut self, role: Duelist, session_id: SessionId) -> Result<(), ArenaError> {
        let slot = match role {
            Duelist::Challenger => &mut self.challenger,
            Duelist::Defender => &mut self.defender,
        };

        if slot.is_some() {
            return Err(ArenaError::RoleTaken(role.to_string()));
        }

        *slot = Some(session_id);
        Ok(())
    }

    fn get_role(&self, session_id: &SessionId) -> Option<Duelist> {
        if self.challenger.as_ref() == Some(session_id) {
            Some(Duelist::Challenger)
        } else if self.defender.as_ref() == Some(session_id) {
            Some(Duelist::Defender)
        } else {
            None
        }
    }

    fn validate_turn(&self, actor: Duelist, session_id: &SessionId) -> Result<(), ArenaError> {
        let expected_session = match actor {
            Duelist::Challenger => self.challenger.as_ref(),
            Duelist::Defender => self.defender.as_ref(),
        };

        if expected_session.is_some_and(|expected| expected != session_id) {
            return Err(ArenaError::NotYourTurn(actor.to_string()));
        }
        Ok(())
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

    /// Starts a new duel, resetting any existing state.
    ///
    /// # Examples
    ///
    /// ```
    /// use insult_arena_mcp::{Arena, ArenaOutcome};
    ///
    /// let mut arena = Arena::new();
    /// let (outcome, view) = arena.start_duel().unwrap();
    ///
    /// assert_eq!(outcome, ArenaOutcome::DuelStarted);
    /// assert_eq!(view.phase, "awaiting_insult");
    /// ```
    ///
    /// # Errors
    /// Returns error if a duel is already in progress and not finished.
    pub fn start_duel(&mut self) -> Result<(ArenaOutcome, DuelStateView), ArenaError> {
        if self.duel.as_ref().is_some_and(|duel| !duel.is_finished()) {
            return Err(ArenaError::DuelInProgress);
        }

        let duel = Duel::new();
        let view = DuelStateView::from(&duel);
        self.duel = Some(duel);

        // Clear session registrations for new duel
        self.sessions = DuelSessions::default();

        Ok((ArenaOutcome::DuelStarted, view))
    }

    /// Register a session for a specific role.
    ///
    /// # Errors
    /// Returns error if the role is already taken or session ID is invalid.
    pub fn register(
        &mut self,
        role: Duelist,
        session_id: SessionId,
    ) -> Result<(ArenaOutcome, Option<DuelStateView>), ArenaError> {
        self.sessions.register(role, session_id)?;
        let state = self.duel.as_ref().map(DuelStateView::from);

        Ok((ArenaOutcome::RoleRegistered { role }, state))
    }

    /// Register a session as the challenger.
    ///
    /// # Errors
    /// Returns error if the role is already taken.
    ///
    /// # Examples
    ///
    /// ```
    /// use insult_arena_mcp::{Arena, SessionId};
    ///
    /// let mut arena = Arena::new();
    /// arena.start_duel();
    ///
    /// let result = arena.register_challenger(SessionId::new("session_123").unwrap());
    /// assert!(result.is_ok());
    ///
    /// // Cannot register if already taken
    /// let result = arena.register_challenger(SessionId::new("other_session").unwrap());
    /// assert!(result.is_err());
    /// ```
    pub fn register_challenger(
        &mut self,
        session_id: SessionId,
    ) -> Result<(ArenaOutcome, Option<DuelStateView>), ArenaError> {
        self.register(Duelist::Challenger, session_id)
    }

    /// Register a session as the defender.
    ///
    /// # Errors
    /// Returns error if the role is already taken.
    ///
    /// # Examples
    ///
    /// ```
    /// use insult_arena_mcp::{Arena, SessionId};
    ///
    /// let mut arena = Arena::new();
    /// arena.start_duel();
    ///
    /// let result = arena.register_defender(SessionId::new("session_456").unwrap());
    /// assert!(result.is_ok());
    /// ```
    pub fn register_defender(
        &mut self,
        session_id: SessionId,
    ) -> Result<(ArenaOutcome, Option<DuelStateView>), ArenaError> {
        self.register(Duelist::Defender, session_id)
    }

    #[must_use]
    pub fn get_role_for_session(&self, session_id: &SessionId) -> Option<Duelist> {
        self.sessions.get_role(session_id)
    }

    /// Get the current state of the duel.
    ///
    /// # Errors
    /// Returns error if no duel is in progress.
    pub fn get_duel_state(
        &self,
        session_id: Option<&SessionId>,
    ) -> Result<(DuelStateView, Option<String>), ArenaError> {
        let Some(duel) = self.duel.as_ref() else {
            return Err(ArenaError::NoDuel);
        };

        let view = DuelStateView::from(duel);
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
    /// Returns error if:
    /// - Input is too long (> `MAX_INPUT_LENGTH`).
    /// - No duel is in progress.
    /// - It is not the session's turn.
    /// - The insult is not known (not in the bank).
    ///
    /// # Examples
    ///
    /// ```
    /// use insult_arena_mcp::{Arena, SessionId, PlayerInput};
    ///
    /// let mut arena = Arena::new();
    /// arena.start_duel();
    /// let alice = SessionId::new("alice").unwrap();
    /// arena.register_challenger(alice.clone()).unwrap();
    ///
    /// // Alice throws a valid insult
    /// let insult = PlayerInput::new("You fight like a dairy farmer!").unwrap();
    /// let result = arena.throw_insult(&alice, insult);
    /// assert!(result.is_ok());
    ///
    /// // Alice cannot throw again (now waiting for comeback)
    /// let insult2 = PlayerInput::new("Another insult").unwrap();
    /// let result = arena.throw_insult(&alice, insult2);
    /// assert!(result.is_err());
    /// ```
    pub fn throw_insult(
        &mut self,
        session_id: &SessionId,
        insult: PlayerInput,
    ) -> Result<(ArenaOutcome, DuelStateView), ArenaError> {
        let mut insult_str = insult.0;

        let Some(duel) = self.duel.as_mut() else {
            return Err(ArenaError::NoDuel);
        };

        // Validate turn/role
        if let DuelState::AwaitingInsult { attacker } = duel.state() {
            self.sessions.validate_turn(attacker, session_id)?;
        }

        // ⚡ Bolt Optimization: Zero allocation path!
        // Using throw_insult_ref to avoid cloning the insult string.
        match duel.throw_insult_ref(&insult_str) {
            Ok(canonical) => {
                // Reuse the existing allocation to store the canonical string.
                insult_str.clear();
                insult_str.push_str(canonical);

                let view = DuelStateView::from(&*duel);
                Ok((ArenaOutcome::InsultThrown { insult: insult_str }, view))
            }
            Err(e) => match e {
                InsultCheckError::UnknownInsult => Err(ArenaError::UnknownInsult(insult_str)),
                InsultCheckError::WaitingForComeback => {
                    Err(ArenaError::DuelError(InsultError::WaitingForComeback))
                }
                InsultCheckError::DuelOver => Err(ArenaError::DuelError(InsultError::DuelOver)),
            },
        }
    }

    /// Respond to an insult with a comeback.
    ///
    /// # Errors
    /// Returns error if:
    /// - Input is too long.
    /// - No duel is in progress.
    /// - It is not the session's turn.
    /// - It is not the comeback phase.
    ///
    /// # Examples
    ///
    /// ```
    /// use insult_arena_mcp::{Arena, SessionId, PlayerInput};
    ///
    /// let mut arena = Arena::new();
    /// arena.start_duel();
    /// let alice = SessionId::new("alice").unwrap();
    /// let bob = SessionId::new("bob").unwrap();
    ///
    /// arena.register_challenger(alice.clone()).unwrap();
    /// arena.register_defender(bob.clone()).unwrap();
    ///
    /// let insult = PlayerInput::new("You fight like a dairy farmer!").unwrap();
    /// arena.throw_insult(&alice, insult).unwrap();
    ///
    /// // Bob responds
    /// let comeback = PlayerInput::new("How appropriate. You fight like a cow!").unwrap();
    /// let (outcome, view) = arena.respond(&bob, comeback).unwrap();
    ///
    /// // Bob won the exchange!
    /// assert_eq!(view.defender_score, 1);
    /// ```
    pub fn respond(
        &mut self,
        session_id: &SessionId,
        comeback: PlayerInput,
    ) -> Result<(ArenaOutcome, DuelStateView), ArenaError> {
        let comeback_str = comeback.0;

        let Some(duel) = self.duel.as_mut() else {
            return Err(ArenaError::NoDuel);
        };

        // Validate turn/role
        if let DuelState::AwaitingComeback { attacker } = duel.state() {
            let defender = attacker.opponent();
            self.sessions.validate_turn(defender, session_id)?;
        }

        // ⚡ Bolt Optimization: Move 'comeback' directly to Duel.
        // Zero allocations here (was 1 from &str).
        match duel.respond(comeback_str) {
            Ok(exchange) => {
                let view = DuelStateView::from(&*duel);
                let outcome = ArenaOutcome::ExchangeProcessed { exchange };
                Ok((outcome, view))
            }
            Err(e) => Err(ArenaError::from(e)),
        }
    }

    /// Get a hint for the current pending insult.
    ///
    /// # Errors
    /// Returns error if:
    /// - No duel is in progress or no insult is pending.
    /// - It is not the session's turn to respond.
    pub fn get_hint(&self, session_id: &SessionId) -> Result<(String, String), ArenaError> {
        let Some(duel) = self.duel.as_ref() else {
            return Err(ArenaError::NoDuel);
        };

        // Ensure it is the correct turn (Defender's turn to respond)
        if let DuelState::AwaitingComeback { attacker } = duel.state() {
            let defender = attacker.opponent();
            self.sessions.validate_turn(defender, session_id)?;
        } else {
            // If we are not waiting for a comeback, we can't give a hint
            return Err(ArenaError::NoPendingInsult);
        }

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
        let (outcome, view) = arena.start_duel().unwrap();
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
        arena.start_duel().unwrap();
        let insults = arena.list_insults().unwrap();
        assert!(insults.iter().any(|&s| s.contains("dairy farmer")));
    }

    #[test]
    fn full_exchange_flow() {
        let mut arena = Arena::new();
        arena.start_duel().unwrap();

        let s_p1 = SessionId::new("p1").unwrap();
        let s_p2 = SessionId::new("p2").unwrap();

        // Throw insult
        let (outcome, view) = arena
            .throw_insult(
                &s_p1,
                PlayerInput::new("You fight like a dairy farmer!").unwrap(),
            )
            .unwrap();
        assert!(matches!(outcome, ArenaOutcome::InsultThrown { .. }));
        assert_eq!(view.phase, "awaiting_comeback");

        // Correct comeback
        let (outcome, view) = arena
            .respond(
                &s_p2,
                PlayerInput::new("How appropriate. You fight like a cow!").unwrap(),
            )
            .unwrap();
        assert!(matches!(outcome, ArenaOutcome::ExchangeProcessed { .. }));
        assert_eq!(view.phase, "awaiting_insult"); // Defender attacks next
    }

    #[test]
    fn register_roles() {
        let mut arena = Arena::new();
        let s1 = SessionId::new("session1").unwrap();
        let s2 = SessionId::new("session2").unwrap();
        let s3 = SessionId::new("session3").unwrap();

        let (outcome, _) = arena.register_challenger(s1).unwrap();
        assert!(matches!(
            outcome,
            ArenaOutcome::RoleRegistered {
                role: Duelist::Challenger
            }
        ));

        let (outcome, _) = arena.register_defender(s2).unwrap();
        assert!(matches!(
            outcome,
            ArenaOutcome::RoleRegistered {
                role: Duelist::Defender
            }
        ));

        // Can't register twice
        let result = arena.register_challenger(s3);
        assert!(matches!(result, Err(ArenaError::RoleTaken(_))));
    }

    #[test]
    fn rejects_excessive_input_length() {
        // Now enforced by PlayerInput::new
        let long_string = "a".repeat(5000);
        let result = PlayerInput::new(long_string);

        assert!(matches!(result, Err(ArenaError::InputTooLong(_))));
    }

    #[test]
    fn rejects_excessive_session_id_length() {
        // Now enforced by SessionId::new
        let long_id = "s".repeat(200);

        let result = SessionId::new(long_id);
        assert!(matches!(result, Err(ArenaError::SessionIdTooLong(_))));
    }

    #[test]
    fn boundary_check_limits() {
        // Check SessionId
        let max_id = "s".repeat(MAX_SESSION_ID_LENGTH);
        assert!(SessionId::new(max_id).is_ok());

        let too_long_id = "s".repeat(MAX_SESSION_ID_LENGTH + 1);
        assert!(matches!(
            SessionId::new(too_long_id),
            Err(ArenaError::SessionIdTooLong(_))
        ));

        // Check PlayerInput
        let max_input = "a".repeat(MAX_INPUT_LENGTH);
        assert!(PlayerInput::new(max_input).is_ok());

        let too_long_input = "a".repeat(MAX_INPUT_LENGTH + 1);
        assert!(matches!(
            PlayerInput::new(too_long_input),
            Err(ArenaError::InputTooLong(_))
        ));
    }

    #[test]
    fn allow_self_play() {
        // Verify one session can play both roles
        let mut arena = Arena::new();

        let session = SessionId::new("solo_player").unwrap();
        assert!(arena.register_challenger(session.clone()).is_ok());
        assert!(arena.register_defender(session.clone()).is_ok());

        assert_eq!(
            arena.get_role_for_session(&session),
            Some(Duelist::Challenger)
        );

        arena.start_duel().unwrap();

        // 1. Throw insult as Challenger (should work)
        let (outcome, _) = arena
            .throw_insult(
                &session,
                PlayerInput::new("You fight like a dairy farmer!").unwrap(),
            )
            .unwrap();
        assert!(matches!(outcome, ArenaOutcome::InsultThrown { .. }));

        // 2. Respond as Defender (should work)
        let (outcome, _) = arena
            .respond(
                &session,
                PlayerInput::new("How appropriate. You fight like a cow!").unwrap(),
            )
            .unwrap();
        assert!(matches!(outcome, ArenaOutcome::ExchangeProcessed { .. }));
    }

    #[test]
    fn beggar_manners_insult_exchange_works() {
        let mut arena = Arena::new();
        arena.start_duel().unwrap();

        let p1 = SessionId::new("p1").unwrap();
        let p2 = SessionId::new("p2").unwrap();

        // Throw "beggar manners" insult
        let (_, view) = arena
            .throw_insult(
                &p1,
                PlayerInput::new("You have the manners of a beggar.").unwrap(),
            )
            .unwrap();
        assert_eq!(view.phase, "awaiting_comeback");

        // Respond with correct comeback
        let (_, view) = arena
            .respond(
                &p2,
                PlayerInput::new("I wanted to make sure you'd feel comfortable with me.").unwrap(),
            )
            .unwrap();
        assert_eq!(view.phase, "awaiting_insult"); // Defender attacks next
    }

    #[test]
    fn throw_insult_without_duel_returns_error() {
        let mut arena = Arena::new();
        let p1 = SessionId::new("p1").unwrap();
        let input = PlayerInput::new("foo").unwrap();
        let result = arena.throw_insult(&p1, input);
        assert_eq!(result.unwrap_err(), ArenaError::NoDuel);
    }

    #[test]
    fn respond_without_duel_returns_error() {
        let mut arena = Arena::new();
        let p1 = SessionId::new("p1").unwrap();
        let input = PlayerInput::new("bar").unwrap();
        let result = arena.respond(&p1, input);
        assert_eq!(result.unwrap_err(), ArenaError::NoDuel);
    }

    #[test]
    fn state_mismatch_errors_are_reported() {
        let mut arena = Arena::new();
        arena.start_duel().unwrap();

        let p1 = SessionId::new("p1").unwrap();
        let p2 = SessionId::new("p2").unwrap();

        let insult_txt = "You fight like a dairy farmer!";
        let comeback_txt = "How appropriate. You fight like a cow!";

        // 1. Throw insult -> OK
        arena
            .throw_insult(&p1, PlayerInput::new(insult_txt).unwrap())
            .unwrap();

        // 2. Throw insult AGAIN -> Error (Waiting for comeback)
        let result = arena.throw_insult(&p1, PlayerInput::new(insult_txt).unwrap());
        assert!(matches!(
            result,
            Err(ArenaError::DuelError(InsultError::WaitingForComeback))
        ));

        // 3. Respond -> OK (Parried, Defender becomes attacker)
        arena
            .respond(&p2, PlayerInput::new(comeback_txt).unwrap())
            .unwrap();

        // 4. Respond AGAIN -> Error (Waiting for insult)
        let result = arena.respond(&p2, PlayerInput::new("Too late").unwrap());
        assert!(matches!(
            result,
            Err(ArenaError::DuelError(InsultError::WaitingForInsult))
        ));
    }

    #[test]
    fn actions_after_duel_finished_return_error() {
        let mut arena = Arena::new();
        arena.start_duel().unwrap();

        let p1 = SessionId::new("p1").unwrap();
        let p2 = SessionId::new("p2").unwrap();

        // Win the duel (Challenger wins 3 times)
        for _ in 0..3 {
            arena
                .throw_insult(&p1, PlayerInput::new("You fight like a dairy farmer!").unwrap())
                .unwrap();
            arena.respond(&p2, PlayerInput::new("wrong").unwrap()).unwrap();
        }

        // Duel should be finished
        let (view, _) = arena.get_duel_state(None).unwrap();
        assert_eq!(view.phase, "finished");

        // Throw insult -> Error
        let result = arena.throw_insult(
            &p1,
            PlayerInput::new("You fight like a dairy farmer!").unwrap(),
        );
        assert!(matches!(
            result,
            Err(ArenaError::DuelError(InsultError::DuelOver))
        ));

        // Respond -> Error
        let result = arena.respond(&p2, PlayerInput::new("wrong").unwrap());
        assert!(matches!(
            result,
            Err(ArenaError::DuelError(InsultError::DuelOver))
        ));
    }

    #[test]
    fn hint_security_check() {
        let mut arena = Arena::new();
        arena.start_duel().unwrap();
        let attacker = SessionId::new("attacker").unwrap();
        let defender = SessionId::new("defender").unwrap();
        let observer = SessionId::new("observer").unwrap();

        arena.register_challenger(attacker.clone()).unwrap();
        arena.register_defender(defender.clone()).unwrap();

        // 1. No one can get hint before insult thrown (NoPendingInsult)
        assert!(matches!(
            arena.get_hint(&attacker),
            Err(ArenaError::NoPendingInsult)
        ));

        // Throw insult
        arena
            .throw_insult(
                &attacker,
                PlayerInput::new("You fight like a dairy farmer!").unwrap(),
            )
            .unwrap();

        // 2. Attacker cannot get hint (NotYourTurn) - Preventing info leak
        assert!(matches!(
            arena.get_hint(&attacker),
            Err(ArenaError::NotYourTurn(_))
        ));

        // 3. Observer cannot get hint
        assert!(matches!(
            arena.get_hint(&observer),
            Err(ArenaError::NotYourTurn(_))
        ));

        // 4. Defender CAN get hint
        let result = arena.get_hint(&defender);
        assert!(result.is_ok());
        let (hint, _) = result.unwrap();
        assert!(!hint.is_empty());
    }

    #[test]
    fn test_turn_enforcement_and_role_protection() {
        let mut arena = Arena::new();
        arena.start_duel().unwrap();

        let alice = SessionId::new("alice").unwrap();
        let bob = SessionId::new("bob").unwrap();
        let eve = SessionId::new("eve").unwrap();

        // Register roles
        arena.register_challenger(alice.clone()).unwrap();
        arena.register_defender(bob.clone()).unwrap();

        let insult = PlayerInput::new("You fight like a dairy farmer!").unwrap();
        let comeback = PlayerInput::new("How appropriate. You fight like a cow!").unwrap();

        // 1. Intruder cannot throw insult
        let result = arena.throw_insult(&eve, insult.clone());
        assert!(matches!(result, Err(ArenaError::NotYourTurn(_))));

        // 2. Defender cannot throw insult (it's Challenger's turn)
        let result = arena.throw_insult(&bob, insult.clone());
        assert!(matches!(result, Err(ArenaError::NotYourTurn(_))));

        // 3. Challenger CAN throw insult
        let (_, view) = arena.throw_insult(&alice, insult.clone()).unwrap();
        assert_eq!(view.phase, "awaiting_comeback");

        // 4. Intruder cannot respond
        let result = arena.respond(&eve, comeback.clone());
        assert!(matches!(result, Err(ArenaError::NotYourTurn(_))));

        // 5. Challenger cannot respond (it's Defender's turn)
        let result = arena.respond(&alice, comeback.clone());
        assert!(matches!(result, Err(ArenaError::NotYourTurn(_))));

        // 6. Defender CAN respond
        let (_, view) = arena.respond(&bob, comeback).unwrap();
        assert_eq!(view.phase, "awaiting_insult");

        // Now Defender is attacker.

        // 7. Challenger cannot throw insult (now Defender's turn)
        let result = arena.throw_insult(&alice, insult.clone());
        assert!(matches!(result, Err(ArenaError::NotYourTurn(_))));

        // 8. Defender CAN throw insult
        let (outcome, _) = arena.throw_insult(&bob, insult).unwrap();
        assert!(matches!(outcome, ArenaOutcome::InsultThrown { .. }));
    }
}
