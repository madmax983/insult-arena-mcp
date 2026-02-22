//! Session management for the Arena.
//!
//! Handles tracking of which session is playing which role (Challenger/Defender).

use super::types::{ArenaError, SessionId};
use crate::duel::Duelist;

/// Tracks which session is playing which role.
///
/// This internal struct maps the abstract roles ([`Duelist::Challenger`] and [`Duelist::Defender`])
/// to concrete session IDs provided by the MCP runtime.
#[derive(Debug, Default)]
pub struct DuelSessions {
    challenger: Option<SessionId>,
    defender: Option<SessionId>,
}

impl DuelSessions {
    pub fn register(&mut self, role: Duelist, session_id: SessionId) -> Result<(), ArenaError> {
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

    pub fn get_role(&self, session_id: &SessionId) -> Option<Duelist> {
        if self.challenger.as_ref() == Some(session_id) {
            Some(Duelist::Challenger)
        } else if self.defender.as_ref() == Some(session_id) {
            Some(Duelist::Defender)
        } else {
            None
        }
    }

    pub fn validate_turn(&self, actor: Duelist, session_id: &SessionId) -> Result<(), ArenaError> {
        let expected_session = match actor {
            Duelist::Challenger => self.challenger.as_ref(),
            Duelist::Defender => self.defender.as_ref(),
        };

        // Sentry Check: Ensure the role is actually registered!
        // Prevents unregistered sessions from hijacking an empty role.
        let Some(expected) = expected_session else {
            return Err(ArenaError::NotYourTurn(actor.to_string()));
        };

        if expected != session_id {
            return Err(ArenaError::NotYourTurn(actor.to_string()));
        }
        Ok(())
    }
}
