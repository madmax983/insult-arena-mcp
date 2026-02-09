//! Duel state machine for insult sword fighting.
//!
//! # Hero's Journey (Example Duel)
//!
//! ```
//! use insult_arena_mcp::{Duel, Duelist, ExchangeResult};
//!
//! // 1. Start a new duel (Challenger vs Defender)
//! let mut duel = Duel::new();
//!
//! // 2. Challenger throws the first insult
//! duel.throw_insult("You fight like a dairy farmer!".to_string()).unwrap();
//!
//! // 3. Defender parries with the correct comeback
//! let exchange = duel.respond("How appropriate. You fight like a cow!".to_string()).unwrap();
//!
//! // 4. Defender won the exchange and is now the attacker!
//! assert!(exchange.result.is_parried());
//! assert_eq!(exchange.winner, Duelist::Defender);
//!
//! // 5. Defender throws the next insult
//! duel.throw_insult("You have the manners of a beggar.".to_string()).unwrap();
//! ```

use serde::{Deserialize, Serialize};

use crate::InsultBank;

/// Identifies a duelist in the fight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Duelist {
    /// The challenger who initiated the duel.
    Challenger,
    /// The defender who accepted the challenge.
    Defender,
}

impl Duelist {
    /// Returns the opponent of this duelist.
    #[must_use]
    pub const fn opponent(self) -> Self {
        match self {
            Self::Challenger => Self::Defender,
            Self::Defender => Self::Challenger,
        }
    }
}

impl std::fmt::Display for Duelist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Challenger => write!(f, "Challenger"),
            Self::Defender => write!(f, "Defender"),
        }
    }
}

/// The result of a single exchange.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExchangeResult {
    /// The defender parried with a perfect comeback.
    Parried {
        /// The insult that was thrown.
        insult: String,
        /// The comeback that defeated it.
        comeback: String,
    },
    /// The defender failed to counter the insult.
    Failed {
        /// The insult that was thrown.
        insult: String,
        /// The failed comeback attempt.
        attempt: String,
        /// The correct comeback they should have used.
        correct: String,
    },
}

impl ExchangeResult {
    /// Returns true if the defender successfully parried.
    #[must_use]
    pub const fn is_parried(&self) -> bool {
        matches!(self, Self::Parried { .. })
    }
}

/// A single exchange in the duel (insult + response).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exchange {
    /// Who threw the insult.
    pub attacker: Duelist,
    /// The result of the exchange.
    pub result: ExchangeResult,
    /// Who won this exchange.
    pub winner: Duelist,
}

/// The current state of a duel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DuelState {
    /// Waiting for an insult to be thrown.
    AwaitingInsult {
        /// Who should throw the insult.
        attacker: Duelist,
    },
    /// Waiting for a comeback response.
    AwaitingComeback {
        /// Who threw the insult.
        attacker: Duelist,
    },
    /// The duel is over.
    Finished {
        /// Who won the duel.
        winner: Duelist,
    },
}

/// The final result of a completed duel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuelResult {
    /// The winner of the duel.
    pub winner: Duelist,
    /// Final score for the challenger.
    pub challenger_score: u8,
    /// Final score for the defender.
    pub defender_score: u8,
    /// All exchanges that occurred.
    pub exchanges: Vec<Exchange>,
}

/// A sword fighting duel between two opponents.
#[derive(Debug, Clone)]
pub struct Duel {
    /// Current state of the duel.
    state: DuelState,
    /// The current pending insult (if any).
    pending_insult: Option<String>,
    /// Score for challenger.
    challenger_score: u8,
    /// Score for defender.
    defender_score: u8,
    /// Wins needed to win the duel.
    wins_needed: u8,
    /// History of exchanges.
    exchanges: Vec<Exchange>,
    /// The insult bank for validating comebacks.
    insult_bank: InsultBank,
}

impl Duel {
    /// Creates a new duel. Challenger attacks first.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: DuelState::AwaitingInsult {
                attacker: Duelist::Challenger,
            },
            pending_insult: None,
            challenger_score: 0,
            defender_score: 0,
            wins_needed: 3,
            exchanges: Vec::new(),
            insult_bank: InsultBank::new(),
        }
    }

    /// Creates a duel with custom wins needed.
    #[must_use]
    pub fn with_wins_needed(wins_needed: u8) -> Self {
        Self {
            wins_needed: std::cmp::max(1, wins_needed),
            ..Self::new()
        }
    }

    /// Returns the current state of the duel.
    #[must_use]
    pub const fn state(&self) -> DuelState {
        self.state
    }

    /// Returns the current scores.
    #[must_use]
    pub const fn scores(&self) -> (u8, u8) {
        (self.challenger_score, self.defender_score)
    }

    /// Returns the number of wins needed to win the duel.
    #[must_use]
    pub const fn wins_needed(&self) -> u8 {
        self.wins_needed
    }

    /// Returns the pending insult if waiting for a comeback.
    #[must_use]
    pub fn pending_insult(&self) -> Option<&str> {
        self.pending_insult.as_deref()
    }

    /// Returns the exchange history.
    #[must_use]
    pub fn exchanges(&self) -> &[Exchange] {
        &self.exchanges
    }

    /// Returns the insult bank for reference.
    #[must_use]
    pub const fn insult_bank(&self) -> &InsultBank {
        &self.insult_bank
    }

    /// Throws an insult. Returns error if not the right time.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - It's not the insult phase (waiting for comeback)
    /// - The insult is not in the `InsultBank`
    /// - The duel is already finished
    ///
    /// # Examples
    ///
    /// ```
    /// use insult_arena_mcp::{Duel, InsultError};
    /// let mut duel = Duel::new();
    ///
    /// // Valid insult
    /// assert!(duel.throw_insult("You fight like a dairy farmer!".to_string()).is_ok());
    ///
    /// // Invalid insult (requires a fresh duel or reset state)
    /// let mut duel2 = Duel::new();
    /// assert!(matches!(
    ///     duel2.throw_insult("Your mother was a hamster!".to_string()),
    ///     Err(InsultError::UnknownInsult(_))
    /// ));
    /// ```
    pub fn throw_insult(&mut self, insult: String) -> Result<(), InsultError> {
        let DuelState::AwaitingInsult { attacker } = self.state else {
            return match self.state {
                DuelState::AwaitingComeback { .. } => Err(InsultError::WaitingForComeback),
                DuelState::Finished { .. } => Err(InsultError::DuelOver),
                DuelState::AwaitingInsult { .. } => unreachable!(),
            };
        };

        // Validate insult exists in bank
        if self.insult_bank.find_comeback(&insult).is_none() {
            return Err(InsultError::UnknownInsult(insult));
        }

        self.pending_insult = Some(insult);
        self.state = DuelState::AwaitingComeback { attacker };
        Ok(())
    }

    /// Responds with a comeback. Returns the exchange result.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - It's not the comeback phase (waiting for insult)
    /// - The duel is already finished
    ///
    /// # Examples
    ///
    /// ```
    /// use insult_arena_mcp::{Duel, ExchangeResult};
    /// let mut duel = Duel::new();
    /// duel.throw_insult("You fight like a dairy farmer!".to_string()).unwrap();
    ///
    /// // Correct response
    /// let exchange = duel.respond("How appropriate. You fight like a cow!".to_string()).unwrap();
    /// assert!(exchange.result.is_parried());
    ///
    /// // Failed response
    /// duel.throw_insult("You have the manners of a beggar.".to_string()).unwrap();
    /// let exchange = duel.respond("I'm rubber you're glue".to_string()).unwrap();
    /// assert!(!exchange.result.is_parried());
    /// ```
    pub fn respond(&mut self, comeback: String) -> Result<Exchange, InsultError> {
        let (attacker, insult) = self.validate_respond_phase()?;
        let defender = attacker.opponent();

        let (result, winner) = self.resolve_exchange(insult, comeback, attacker, defender);

        self.update_scores(winner);

        let exchange = Exchange {
            attacker,
            result,
            winner,
        };
        self.exchanges.push(exchange.clone());

        self.update_state_after_exchange(winner);

        Ok(exchange)
    }

    fn validate_respond_phase(&mut self) -> Result<(Duelist, String), InsultError> {
        let attacker = match self.state {
            DuelState::AwaitingComeback { attacker } => attacker,
            DuelState::AwaitingInsult { .. } => return Err(InsultError::WaitingForInsult),
            DuelState::Finished { .. } => return Err(InsultError::DuelOver),
        };

        let insult = self
            .pending_insult
            .take()
            .ok_or(InsultError::WaitingForInsult)?;

        Ok((attacker, insult))
    }

    fn resolve_exchange(
        &self,
        insult: String,
        comeback: String,
        attacker: Duelist,
        defender: Duelist,
    ) -> (ExchangeResult, Duelist) {
        let is_correct = self
            .insult_bank
            .check_comeback(&insult, &comeback)
            .is_some();

        if is_correct {
            // Successful parry! Defender wins exchange and becomes attacker.
            (ExchangeResult::Parried { insult, comeback }, defender)
        } else {
            // Failed comeback. Attacker wins exchange.
            let correct = self
                .insult_bank
                .find_comeback(&insult)
                .unwrap_or("???")
                .to_string();
            (
                ExchangeResult::Failed {
                    insult,
                    attempt: comeback,
                    correct,
                },
                attacker,
            )
        }
    }

    const fn update_scores(&mut self, winner: Duelist) {
        match winner {
            Duelist::Challenger => self.challenger_score = self.challenger_score.saturating_add(1),
            Duelist::Defender => self.defender_score = self.defender_score.saturating_add(1),
        }
    }

    const fn update_state_after_exchange(&mut self, winner: Duelist) {
        if self.challenger_score >= self.wins_needed {
            self.state = DuelState::Finished {
                winner: Duelist::Challenger,
            };
        } else if self.defender_score >= self.wins_needed {
            self.state = DuelState::Finished {
                winner: Duelist::Defender,
            };
        } else {
            // Winner of exchange becomes the attacker
            self.state = DuelState::AwaitingInsult { attacker: winner };
        }
    }

    /// Gets the final result if the duel is over.
    #[must_use]
    pub fn result(&self) -> Option<DuelResult> {
        match self.state {
            DuelState::Finished { winner } => Some(DuelResult {
                winner,
                challenger_score: self.challenger_score,
                defender_score: self.defender_score,
                exchanges: self.exchanges.clone(),
            }),
            _ => None,
        }
    }

    /// Returns true if the duel is finished.
    #[must_use]
    pub const fn is_finished(&self) -> bool {
        matches!(self.state, DuelState::Finished { .. })
    }
}

impl Default for Duel {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during a duel.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InsultError {
    /// The insult is not recognized.
    #[error("Unknown insult: {0}")]
    UnknownInsult(String),
    /// It's not time for an insult, we're waiting for a comeback.
    #[error("Waiting for a comeback, not an insult")]
    WaitingForComeback,
    /// It's not time for a comeback, we're waiting for an insult.
    #[error("Waiting for an insult, not a comeback")]
    WaitingForInsult,
    /// The duel is already over.
    #[error("The duel is over")]
    DuelOver,
    /// No active duel.
    #[error("No active duel - challenge someone first")]
    NoDuel,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn new_duel_awaits_challenger_insult() {
        let duel = Duel::new();
        assert_eq!(
            duel.state(),
            DuelState::AwaitingInsult {
                attacker: Duelist::Challenger
            }
        );
    }

    #[test]
    fn throw_insult_transitions_to_awaiting_comeback() {
        let mut duel = Duel::new();
        duel.throw_insult("You fight like a dairy farmer!".to_string())
            .unwrap();
        assert_eq!(
            duel.state(),
            DuelState::AwaitingComeback {
                attacker: Duelist::Challenger
            }
        );
    }

    #[test]
    fn correct_comeback_parries() {
        let mut duel = Duel::new();
        duel.throw_insult("You fight like a dairy farmer!".to_string())
            .unwrap();
        let exchange = duel
            .respond("How appropriate. You fight like a cow!".to_string())
            .unwrap();

        assert!(exchange.result.is_parried());
        assert_eq!(exchange.winner, Duelist::Defender);
    }

    #[test]
    fn wrong_comeback_fails() {
        let mut duel = Duel::new();
        duel.throw_insult("You fight like a dairy farmer!".to_string())
            .unwrap();
        let exchange = duel.respond("No u".to_string()).unwrap();

        assert!(!exchange.result.is_parried());
        assert_eq!(exchange.winner, Duelist::Challenger);
    }

    #[test]
    fn winner_becomes_next_attacker() {
        let mut duel = Duel::new();
        duel.throw_insult("You fight like a dairy farmer!".to_string())
            .unwrap();
        duel.respond("How appropriate. You fight like a cow!".to_string())
            .unwrap();

        // Defender won, so defender attacks next
        assert_eq!(
            duel.state(),
            DuelState::AwaitingInsult {
                attacker: Duelist::Defender
            }
        );
    }

    #[test]
    fn first_to_three_wins() {
        let mut duel = Duel::new();

        // Challenger wins 3 exchanges
        for _ in 0..3 {
            duel.throw_insult("You fight like a dairy farmer!".to_string())
                .unwrap();
            duel.respond("wrong answer".to_string()).unwrap();
        }

        assert!(duel.is_finished());
        let result = duel.result().unwrap();
        assert_eq!(result.winner, Duelist::Challenger);
        assert_eq!(result.challenger_score, 3);
        assert_eq!(result.defender_score, 0);
    }

    #[test]
    fn unknown_insult_rejected() {
        let mut duel = Duel::new();
        let result = duel.throw_insult("Your mother was a hamster!".to_string());
        assert!(matches!(result, Err(InsultError::UnknownInsult(_))));
    }

    #[test]
    fn cannot_throw_insult_when_awaiting_comeback() {
        let mut duel = Duel::new();
        duel.throw_insult("You fight like a dairy farmer!".to_string())
            .unwrap();
        let result =
            duel.throw_insult("People fall at my feet when they see me coming!".to_string());
        assert!(matches!(result, Err(InsultError::WaitingForComeback)));
    }

    #[test]
    fn cannot_respond_when_awaiting_insult() {
        let mut duel = Duel::new();
        let result = duel.respond("How appropriate. You fight like a cow.".to_string());
        assert!(matches!(result, Err(InsultError::WaitingForInsult)));
    }

    #[test]
    fn duelist_opponent() {
        assert_eq!(Duelist::Challenger.opponent(), Duelist::Defender);
        assert_eq!(Duelist::Defender.opponent(), Duelist::Challenger);
    }

    #[test]
    fn scores_update_correctly() {
        let mut duel = Duel::new();
        assert_eq!(duel.scores(), (0, 0));

        duel.throw_insult("You fight like a dairy farmer!".to_string())
            .unwrap();
        duel.respond("wrong".to_string()).unwrap();
        assert_eq!(duel.scores(), (1, 0));

        duel.throw_insult("You fight like a dairy farmer!".to_string())
            .unwrap();
        duel.respond("How appropriate. You fight like a cow!".to_string())
            .unwrap();
        assert_eq!(duel.scores(), (1, 1));
    }

    #[test]
    fn beggar_manners_insult_correct_comeback_parries() {
        let mut duel = Duel::new();
        duel.throw_insult("You have the manners of a beggar.".to_string())
            .unwrap();
        let exchange = duel
            .respond("I wanted to make sure you'd feel comfortable with me.".to_string())
            .unwrap();

        assert!(
            exchange.result.is_parried(),
            "Comeback should parry the insult"
        );
        assert_eq!(
            exchange.winner,
            Duelist::Defender,
            "Defender should win with correct comeback"
        );
    }

    #[test]
    fn custom_wins_needed_works() {
        let mut duel = Duel::with_wins_needed(1);
        assert_eq!(duel.wins_needed(), 1);

        duel.throw_insult("You fight like a dairy farmer!".to_string())
            .unwrap();
        // Respond incorrectly -> Challenger wins exchange -> Challenger wins duel (score 1)
        duel.respond("wrong answer".to_string()).unwrap();

        assert!(duel.is_finished());
        let result = duel.result().unwrap();
        assert_eq!(result.winner, Duelist::Challenger);
        assert_eq!(result.challenger_score, 1);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod sentry_repro_tests {
    use super::*;

    #[test]
    fn zero_wins_needed_bug_reproduction() {
        // If wins_needed is 0, the game logic is flawed.
        // Even if Defender wins the first exchange (1-0), Challenger has 0 >= 0, so Challenger wins?
        let mut duel = Duel::with_wins_needed(0);
        duel.throw_insult("You fight like a dairy farmer!".to_string())
            .unwrap();

        // Defender wins the exchange
        let exchange = duel
            .respond("How appropriate. You fight like a cow!".to_string())
            .unwrap();

        assert!(exchange.result.is_parried());
        assert_eq!(exchange.winner, Duelist::Defender);
        assert_eq!(duel.scores(), (0, 1)); // Defender has 1 point

        // CHECK BUG FIXED:
        // wins_needed should be clamped to 1.
        // Defender score is 1, Challenger is 0.
        // Defender should win.
        if duel.is_finished() {
            let result = duel.result().unwrap();
            assert_eq!(
                result.winner,
                Duelist::Defender,
                "Fixed: Defender wins because wins_needed clamped to 1"
            );
        } else {
            panic!("Duel should be finished because wins_needed is 1 and score is 1");
        }
    }

    #[test]
    fn max_wins_needed_saturation() {
        // Test that we can reach the maximum possible score (255) without panicking
        let mut duel = Duel::with_wins_needed(255);

        let insult = "You fight like a dairy farmer!".to_string();
        let wrong_comeback = "wrong".to_string();

        // 1. Reach 254 wins
        for _ in 0..254 {
            duel.throw_insult(insult.clone()).unwrap();
            let exchange = duel.respond(wrong_comeback.clone()).unwrap();
            assert_eq!(exchange.winner, Duelist::Challenger);
        }

        let (c_score, _) = duel.scores();
        assert_eq!(c_score, 254);
        assert!(!duel.is_finished());

        // 2. Reach 255 wins (Match Point!)
        duel.throw_insult(insult).unwrap();
        let exchange = duel.respond(wrong_comeback).unwrap();
        assert_eq!(exchange.winner, Duelist::Challenger);

        let (c_score, _) = duel.scores();
        assert_eq!(c_score, 255);
        assert!(duel.is_finished());

        let result = duel.result().unwrap();
        assert_eq!(result.winner, Duelist::Challenger);
        assert_eq!(result.challenger_score, 255);
    }
}
