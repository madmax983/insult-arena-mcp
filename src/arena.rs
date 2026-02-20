//! Arena module: Encapsulates the game state and logic.
//!
//! This module also enforces security constraints to prevent Denial of Service (DoS):
//! - **Input Length Limits**: Insults and session IDs are checked against [`MAX_INPUT_LENGTH`] and [`MAX_SESSION_ID_LENGTH`].
//! - **Timeouts**: Stale duels are automatically reset to free up resources.
//!
//! # Hero's Journey
//!
//! ```
//! use insult_arena_mcp::{Arena, ArenaOutcome};
//! use insult_arena_mcp::arena::{PlayerInput, SessionId};
//!
//! // 1. Create the Arena
//! let mut arena = Arena::new();
//!
//! // 2. Start a new duel
//! let (outcome, view) = arena.start_duel().unwrap();
//! assert_eq!(view.phase, "awaiting_insult");
//!
//! // 3. Register players
//! let alice = SessionId::try_from("session_A".to_string()).unwrap();
//! let bob = SessionId::try_from("session_B".to_string()).unwrap();
//!
//! arena.register_challenger(alice.clone()).unwrap();
//! arena.register_defender(bob.clone()).unwrap();
//!
//! // 4. Challenger throws an insult
//! let insult = PlayerInput::try_from("You fight like a dairy farmer!".to_string()).unwrap();
//! let (outcome, view) = arena.throw_insult(&alice, insult).unwrap();
//!
//! // 5. Defender responds
//! let comeback = PlayerInput::try_from("How appropriate. You fight like a cow!".to_string()).unwrap();
//! let (outcome, view) = arena.respond(&bob, comeback).unwrap();
//!
//! // Defender won the exchange!
//! assert_eq!(view.defender_score, 1);
//! ```

use std::time::{Duration, Instant};

use tracing::warn;

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

/// A validated session ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(String);

impl SessionId {
    /// Returns the session ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SessionId {
    type Error = ArenaError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() > MAX_SESSION_ID_LENGTH {
            Err(ArenaError::SessionIdTooLong(MAX_SESSION_ID_LENGTH))
        } else {
            Ok(Self(value))
        }
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

/// A validated player input string (insult or comeback).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerInput(String);

impl PlayerInput {
    /// Returns the input as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the wrapper and returns the inner String.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl TryFrom<String> for PlayerInput {
    type Error = ArenaError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() > MAX_INPUT_LENGTH {
            Err(ArenaError::InputTooLong(MAX_INPUT_LENGTH))
        } else {
            Ok(Self(value))
        }
    }
}

impl AsRef<str> for PlayerInput {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Errors that can occur in the Arena.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ArenaError {
    /// No duel is currently active.
    #[error("No duel in progress. Call start_duel first!")]
    NoDuel,
    /// User input exceeded the maximum allowed length.
    #[error("Input too long (max {0} chars)")]
    InputTooLong(usize),
    /// Session ID exceeded the maximum allowed length.
    #[error("Session ID too long (max {0} chars)")]
    SessionIdTooLong(usize),
    /// The requested role is already occupied by another session.
    #[error("{0} role is already taken!")]
    RoleTaken(String),
    /// The session tried to act out of turn.
    #[error("It is not your turn! Waiting for {0}.")]
    NotYourTurn(String),
    /// The insult is not in the [`crate::InsultBank`].
    #[error("Unknown insult: \"{0}\". Use list_insults to see valid options.")]
    UnknownInsult(String),
    /// A hint was requested but no insult is pending.
    #[error("No pending insult to hint about.")]
    NoPendingInsult,
    /// Could not find a comeback for the pending insult (should not happen with valid insults).
    #[error("Could not find comeback for this insult.")]
    ComebackNotFound,
    /// Tried to start a duel while one is already in progress.
    #[error("A duel is already in progress. Wait for it to finish!")]
    DuelInProgress,
    /// An error from the underlying duel logic.
    #[error(transparent)]
    DuelError(#[from] InsultError),
}

/// The outcome of an action in the Arena.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArenaOutcome {
    /// A new duel has started.
    DuelStarted,
    /// A session has successfully registered for a role.
    RoleRegistered {
        /// The role that was assigned.
        role: Duelist,
    },
    /// An insult was successfully thrown.
    InsultThrown {
        /// The insult that was thrown.
        insult: String,
    },
    /// An exchange (insult + comeback) was completed.
    ExchangeProcessed {
        /// The result of the exchange.
        exchange: crate::duel::Exchange,
    },
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
    /// Timestamp of the last successful action.
    last_active: Instant,
    /// Duration after which an active duel is considered stale and can be reset.
    timeout: Duration,
}

impl Arena {
    /// Creates a new, empty Arena.
    ///
    /// # Examples
    ///
    /// ```
    /// use insult_arena_mcp::Arena;
    /// let arena = Arena::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            duel: None,
            sessions: DuelSessions::default(),
            last_active: Instant::now(),
            timeout: Duration::from_secs(300), // 5 minutes default
        }
    }

    /// Set a custom timeout for the duel.
    ///
    /// Useful for testing or adjusting game pace.
    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    fn validate_insult_turn(
        sessions: &DuelSessions,
        duel: &Duel,
        session_id: &SessionId,
    ) -> Result<(), ArenaError> {
        if let DuelState::AwaitingInsult { attacker } = duel.state() {
            sessions.validate_turn(attacker, session_id)?;
        }
        Ok(())
    }

    fn validate_response_turn(
        sessions: &DuelSessions,
        duel: &Duel,
        session_id: &SessionId,
    ) -> Result<(), ArenaError> {
        if let DuelState::AwaitingComeback { attacker } = duel.state() {
            let defender = attacker.opponent();
            sessions.validate_turn(defender, session_id)?;
        }
        Ok(())
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
            // Check if the duel is stale (DoS protection)
            if self.last_active.elapsed() > self.timeout {
                warn!("⚠️  Resetting stale duel due to inactivity.");
            } else {
                return Err(ArenaError::DuelInProgress);
            }
        }

        let duel = Duel::new();
        let view = DuelStateView::from(&duel);
        self.duel = Some(duel);
        self.last_active = Instant::now();

        // Clear session registrations for new duel
        self.sessions = DuelSessions::default();

        Ok((ArenaOutcome::DuelStarted, view))
    }

    /// Register a session for a specific role.
    ///
    /// # Errors
    /// Returns error if the role is already taken, session ID is invalid,
    /// or no duel is in progress.
    pub fn register(
        &mut self,
        role: Duelist,
        session_id: SessionId,
    ) -> Result<(ArenaOutcome, DuelStateView), ArenaError> {
        if self.duel.is_none() {
            return Err(ArenaError::NoDuel);
        }

        self.sessions.register(role, session_id)?;
        self.last_active = Instant::now();

        let duel = self.duel.as_ref().ok_or(ArenaError::NoDuel)?;
        let state = DuelStateView::from(duel);

        Ok((ArenaOutcome::RoleRegistered { role }, state))
    }

    /// Register a session as the challenger.
    ///
    /// # Errors
    /// Returns error if the role is already taken or no duel is in progress.
    ///
    /// # Examples
    ///
    /// ```
    /// use insult_arena_mcp::Arena;
    /// use insult_arena_mcp::arena::SessionId;
    ///
    /// let mut arena = Arena::new();
    /// arena.start_duel();
    ///
    /// let sid = SessionId::try_from("session_123".to_string()).unwrap();
    /// let result = arena.register_challenger(sid);
    /// assert!(result.is_ok());
    /// ```
    pub fn register_challenger(
        &mut self,
        session_id: SessionId,
    ) -> Result<(ArenaOutcome, DuelStateView), ArenaError> {
        self.register(Duelist::Challenger, session_id)
    }

    /// Register a session as the defender.
    ///
    /// # Errors
    /// Returns error if the role is already taken or no duel is in progress.
    ///
    /// # Examples
    ///
    /// ```
    /// use insult_arena_mcp::Arena;
    /// use insult_arena_mcp::arena::SessionId;
    ///
    /// let mut arena = Arena::new();
    /// arena.start_duel();
    ///
    /// let sid = SessionId::try_from("session_456".to_string()).unwrap();
    /// let result = arena.register_defender(sid);
    /// assert!(result.is_ok());
    /// ```
    pub fn register_defender(
        &mut self,
        session_id: SessionId,
    ) -> Result<(ArenaOutcome, DuelStateView), ArenaError> {
        self.register(Duelist::Defender, session_id)
    }

    /// Returns the role (if any) associated with the given session ID.
    ///
    /// # Examples
    ///
    /// ```
    /// use insult_arena_mcp::{Arena, Duelist};
    /// use insult_arena_mcp::arena::SessionId;
    ///
    /// let mut arena = Arena::new();
    /// arena.start_duel().unwrap();
    ///
    /// let sid = SessionId::try_from("session_123".to_string()).unwrap();
    /// arena.register_challenger(sid.clone()).unwrap();
    ///
    /// assert_eq!(arena.get_role_for_session(&sid), Some(Duelist::Challenger));
    /// ```
    #[must_use]
    pub fn get_role_for_session(&self, session_id: &SessionId) -> Option<Duelist> {
        self.sessions.get_role(session_id)
    }

    /// Get the current state of the duel.
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// 1. [`DuelStateView`]: The public state of the game (scores, phase, etc).
    /// 2. `Option<String>`: The role of the requested session ("Challenger", "Defender", or None).
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
    /// use insult_arena_mcp::Arena;
    /// use insult_arena_mcp::arena::{PlayerInput, SessionId};
    ///
    /// let mut arena = Arena::new();
    /// arena.start_duel();
    ///
    /// let alice = SessionId::try_from("alice".to_string()).unwrap();
    /// arena.register_challenger(alice.clone()).unwrap();
    ///
    /// // Alice throws a valid insult
    /// let insult = PlayerInput::try_from("You fight like a dairy farmer!".to_string()).unwrap();
    /// let result = arena.throw_insult(&alice, insult);
    /// assert!(result.is_ok());
    /// ```
    pub fn throw_insult(
        &mut self,
        session_id: &SessionId,
        insult: PlayerInput,
    ) -> Result<(ArenaOutcome, DuelStateView), ArenaError> {
        let Some(duel) = self.duel.as_mut() else {
            return Err(ArenaError::NoDuel);
        };

        Self::validate_insult_turn(&self.sessions, duel, session_id)?;

        // ⚡ Bolt Optimization: Zero allocation path!
        // Using throw_insult_ref to avoid cloning the insult string.
        match duel.throw_insult_ref(insult.as_str()) {
            Ok(canonical) => {
                self.last_active = Instant::now();
                // Reuse the existing allocation to store the canonical string.
                let mut insult = insult.into_inner();
                insult.clear();
                insult.push_str(canonical);

                let view = DuelStateView::from(&*duel);
                Ok((ArenaOutcome::InsultThrown { insult }, view))
            }
            Err(e) => Err(Self::map_insult_check_error(e, insult.into_inner())),
        }
    }

    fn map_insult_check_error(e: InsultCheckError, insult: String) -> ArenaError {
        match e {
            InsultCheckError::UnknownInsult => ArenaError::UnknownInsult(insult),
            InsultCheckError::WaitingForComeback => {
                ArenaError::DuelError(InsultError::WaitingForComeback)
            }
            InsultCheckError::DuelOver => ArenaError::DuelError(InsultError::DuelOver),
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
    /// use insult_arena_mcp::Arena;
    /// use insult_arena_mcp::arena::{PlayerInput, SessionId};
    ///
    /// let mut arena = Arena::new();
    /// arena.start_duel();
    ///
    /// let alice = SessionId::try_from("alice".to_string()).unwrap();
    /// let bob = SessionId::try_from("bob".to_string()).unwrap();
    ///
    /// arena.register_challenger(alice.clone()).unwrap();
    /// arena.register_defender(bob.clone()).unwrap();
    ///
    /// let insult = PlayerInput::try_from("You fight like a dairy farmer!".to_string()).unwrap();
    /// arena.throw_insult(&alice, insult).unwrap();
    ///
    /// // Bob responds
    /// let comeback = PlayerInput::try_from("How appropriate. You fight like a cow!".to_string()).unwrap();
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
        let Some(duel) = self.duel.as_mut() else {
            return Err(ArenaError::NoDuel);
        };

        Self::validate_response_turn(&self.sessions, duel, session_id)?;

        // ⚡ Bolt Optimization: Move 'comeback' directly to Duel.
        // Zero allocations here (was 1 from &str).
        match duel.respond(comeback.into_inner()) {
            Ok(exchange) => {
                self.last_active = Instant::now();
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
        let DuelState::AwaitingComeback { attacker } = duel.state() else {
            return Err(ArenaError::NoPendingInsult);
        };

        let defender = attacker.opponent();
        self.sessions.validate_turn(defender, session_id)?;

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

    fn sid(s: &str) -> SessionId {
        SessionId::try_from(s.to_string()).unwrap()
    }

    fn input(s: &str) -> PlayerInput {
        PlayerInput::try_from(s.to_string()).unwrap()
    }

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

        let p1 = sid("p1");
        let p2 = sid("p2");

        // Need to register first now (since throw_insult checks turn)
        arena.register_challenger(p1.clone()).unwrap();
        arena.register_defender(p2.clone()).unwrap();

        // Throw insult
        let (outcome, view) = arena
            .throw_insult(&p1, input("You fight like a dairy farmer!"))
            .unwrap();
        assert!(matches!(outcome, ArenaOutcome::InsultThrown { .. }));
        assert_eq!(view.phase, "awaiting_comeback");

        // Correct comeback
        let (outcome, view) = arena
            .respond(&p2, input("How appropriate. You fight like a cow!"))
            .unwrap();
        assert!(matches!(outcome, ArenaOutcome::ExchangeProcessed { .. }));
        assert_eq!(view.phase, "awaiting_insult"); // Defender attacks next
    }

    #[test]
    fn register_roles() {
        let mut arena = Arena::new();
        arena.start_duel().unwrap();

        let s1 = sid("session1");
        let s2 = sid("session2");
        let s3 = sid("session3");

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
        // Validation now happens at TryFrom
        let long_string = "a".repeat(5000);
        let result = PlayerInput::try_from(long_string);

        assert!(matches!(result, Err(ArenaError::InputTooLong(_))));
    }

    #[test]
    fn rejects_excessive_session_id_length() {
        let long_id = "s".repeat(200);
        let result = SessionId::try_from(long_id);

        assert!(matches!(result, Err(ArenaError::SessionIdTooLong(_))));
    }

    #[test]
    fn boundary_check_limits() {
        // Exact limit should pass (Session ID)
        let max_id = "s".repeat(MAX_SESSION_ID_LENGTH);
        assert!(SessionId::try_from(max_id).is_ok());

        // Limit + 1 should fail
        let too_long_id = "s".repeat(MAX_SESSION_ID_LENGTH + 1);
        assert!(matches!(
            SessionId::try_from(too_long_id),
            Err(ArenaError::SessionIdTooLong(_))
        ));

        // Exact limit should pass (Input)
        let max_input = "a".repeat(MAX_INPUT_LENGTH);
        assert!(PlayerInput::try_from(max_input).is_ok());

        let too_long_input = "a".repeat(MAX_INPUT_LENGTH + 1);
        assert!(matches!(
            PlayerInput::try_from(too_long_input),
            Err(ArenaError::InputTooLong(_))
        ));
    }

    #[test]
    fn allow_self_play() {
        // Verify one session can play both roles
        let mut arena = Arena::new();
        arena.start_duel().unwrap();

        let session = sid("solo_player");
        assert!(arena.register_challenger(session.clone()).is_ok());
        assert!(arena.register_defender(session.clone()).is_ok());

        assert_eq!(
            arena.get_role_for_session(&session),
            Some(Duelist::Challenger)
        );

        // 1. Throw insult as Challenger (should work)
        let (outcome, _) = arena
            .throw_insult(&session, input("You fight like a dairy farmer!"))
            .unwrap();
        assert!(matches!(outcome, ArenaOutcome::InsultThrown { .. }));

        // 2. Respond as Defender (should work)
        let (outcome, _) = arena
            .respond(&session, input("How appropriate. You fight like a cow!"))
            .unwrap();
        assert!(matches!(outcome, ArenaOutcome::ExchangeProcessed { .. }));
    }

    #[test]
    fn beggar_manners_insult_exchange_works() {
        let mut arena = Arena::new();
        arena.start_duel().unwrap();

        let p1 = sid("p1");
        let p2 = sid("p2");
        arena.register_challenger(p1.clone()).unwrap();
        arena.register_defender(p2.clone()).unwrap();

        // Throw "beggar manners" insult
        let (_, view) = arena
            .throw_insult(&p1, input("You have the manners of a beggar."))
            .unwrap();
        assert_eq!(view.phase, "awaiting_comeback");

        // Respond with correct comeback
        let (_, view) = arena
            .respond(
                &p2,
                input("I wanted to make sure you'd feel comfortable with me."),
            )
            .unwrap();
        assert_eq!(view.phase, "awaiting_insult"); // Defender attacks next
    }

    #[test]
    fn throw_insult_without_duel_returns_error() {
        let mut arena = Arena::new();
        let result = arena.throw_insult(&sid("p1"), input("foo"));
        assert_eq!(result.unwrap_err(), ArenaError::NoDuel);
    }

    #[test]
    fn respond_without_duel_returns_error() {
        let mut arena = Arena::new();
        let result = arena.respond(&sid("p1"), input("bar"));
        assert_eq!(result.unwrap_err(), ArenaError::NoDuel);
    }

    #[test]
    fn state_mismatch_errors_are_reported() {
        let mut arena = Arena::new();
        arena.start_duel().unwrap();
        let p1 = sid("p1");
        let p2 = sid("p2");
        arena.register_challenger(p1.clone()).unwrap();
        arena.register_defender(p2.clone()).unwrap();

        // 1. Throw insult -> OK
        arena
            .throw_insult(&p1, input("You fight like a dairy farmer!"))
            .unwrap();

        // 2. Throw insult AGAIN -> Error (Waiting for comeback)
        let result = arena.throw_insult(&p1, input("You fight like a dairy farmer!"));
        assert!(matches!(
            result,
            Err(ArenaError::DuelError(InsultError::WaitingForComeback))
        ));

        // 3. Respond -> OK (Parried, Defender becomes attacker)
        arena
            .respond(&p2, input("How appropriate. You fight like a cow!"))
            .unwrap();

        // 4. Respond AGAIN -> Error (Waiting for insult)
        let result = arena.respond(&p2, input("Too late"));
        assert!(matches!(
            result,
            Err(ArenaError::DuelError(InsultError::WaitingForInsult))
        ));
    }

    #[test]
    fn actions_after_duel_finished_return_error() {
        let mut arena = Arena::new();
        arena.start_duel().unwrap();
        let p1 = sid("p1");
        let p2 = sid("p2");
        arena.register_challenger(p1.clone()).unwrap();
        arena.register_defender(p2.clone()).unwrap();

        // Win the duel (Challenger wins 3 times)
        for _ in 0..3 {
            arena
                .throw_insult(&p1, input("You fight like a dairy farmer!"))
                .unwrap();
            arena.respond(&p2, input("wrong")).unwrap();
        }

        // Duel should be finished
        let (view, _) = arena.get_duel_state(None).unwrap();
        assert_eq!(view.phase, "finished");

        // Throw insult -> Error
        let result = arena.throw_insult(&p1, input("You fight like a dairy farmer!"));
        assert!(matches!(
            result,
            Err(ArenaError::DuelError(InsultError::DuelOver))
        ));

        // Respond -> Error
        let result = arena.respond(&p2, input("wrong"));
        assert!(matches!(
            result,
            Err(ArenaError::DuelError(InsultError::DuelOver))
        ));
    }

    #[test]
    fn hint_security_check() {
        let mut arena = Arena::new();
        arena.start_duel().unwrap();
        let attacker = sid("attacker");
        let defender = sid("defender");
        let observer = sid("observer");

        arena.register_challenger(attacker.clone()).unwrap();
        arena.register_defender(defender.clone()).unwrap();

        // 1. No one can get hint before insult thrown (NoPendingInsult)
        assert!(matches!(
            arena.get_hint(&attacker),
            Err(ArenaError::NoPendingInsult)
        ));

        // Throw insult
        arena
            .throw_insult(&attacker, input("You fight like a dairy farmer!"))
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

        let alice = sid("alice");
        let bob = sid("bob");
        let eve = sid("eve");

        // Register roles
        arena.register_challenger(alice.clone()).unwrap();
        arena.register_defender(bob.clone()).unwrap();

        // 1. Intruder cannot throw insult
        let result = arena.throw_insult(&eve, input("You fight like a dairy farmer!"));
        assert!(matches!(result, Err(ArenaError::NotYourTurn(_))));

        // 2. Defender cannot throw insult (it's Challenger's turn)
        let result = arena.throw_insult(&bob, input("You fight like a dairy farmer!"));
        assert!(matches!(result, Err(ArenaError::NotYourTurn(_))));

        // 3. Challenger CAN throw insult
        let (_, view) = arena
            .throw_insult(&alice, input("You fight like a dairy farmer!"))
            .unwrap();
        assert_eq!(view.phase, "awaiting_comeback");

        // 4. Intruder cannot respond
        let result = arena.respond(&eve, input("How appropriate. You fight like a cow!"));
        assert!(matches!(result, Err(ArenaError::NotYourTurn(_))));

        // 5. Challenger cannot respond (it's Defender's turn)
        let result = arena.respond(&alice, input("How appropriate. You fight like a cow!"));
        assert!(matches!(result, Err(ArenaError::NotYourTurn(_))));

        // 6. Defender CAN respond
        let (_, view) = arena
            .respond(&bob, input("How appropriate. You fight like a cow!"))
            .unwrap();
        assert_eq!(view.phase, "awaiting_insult");

        // Now Defender is attacker.

        // 7. Challenger cannot throw insult (now Defender's turn)
        let result = arena.throw_insult(&alice, input("You fight like a dairy farmer!"));
        assert!(matches!(result, Err(ArenaError::NotYourTurn(_))));

        // 8. Defender CAN throw insult
        let (outcome, _) = arena
            .throw_insult(&bob, input("You fight like a dairy farmer!"))
            .unwrap();
        assert!(matches!(outcome, ArenaOutcome::InsultThrown { .. }));
    }
}
