use crate::{Duel, Duelist, ExchangeResult};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Badges awarded for specific feats during a duel.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Achievement {
    /// Won the duel without taking any damage (Opponent score 0).
    Untouchable,
    /// Successfully parried every insult thrown at them (and parried at least one).
    SharpTongue,
    /// Won the match after facing match point (Opponent needed 1 more win).
    ComebackKid,
    /// Used 3 or more unique insults in a single match.
    Encyclopedia,
    /// Won the duel in the minimum possible number of turns.
    Speedrun,
}

impl Achievement {
    /// Analyzes a completed duel and awards achievements.
    ///
    /// # Arguments
    ///
    /// * `duel` - The completed duel to analyze.
    ///
    /// # Returns
    ///
    /// A list of achievements earned by the **winner**.
    /// Returns empty list if the duel is not finished.
    #[must_use]
    pub fn check(duel: &Duel) -> Vec<Self> {
        let mut achievements = Vec::new();

        let Some(result) = duel.result() else {
            return achievements;
        };

        let winner = result.winner;
        let (winner_score, loser_score) = match winner {
            Duelist::Challenger => (result.challenger_score, result.defender_score),
            Duelist::Defender => (result.defender_score, result.challenger_score),
        };

        // 1. Untouchable
        if loser_score == 0 {
            achievements.push(Self::Untouchable);
        }

        // 2. Encyclopedia
        let unique_insults: HashSet<&str> =
            duel.exchanges()
                .iter()
                .map(|e| match &e.result {
                    ExchangeResult::Parried { insult, .. }
                    | ExchangeResult::Failed { insult, .. } => insult.as_str(),
                })
                .collect();
        if unique_insults.len() >= 3 {
            achievements.push(Self::Encyclopedia);
        }

        // 3. Sharp Tongue
        // Check if winner ever missed a parry *when they were defending*.
        // They defend when they are NOT the attacker.
        // Wait, Exchange struct has `attacker`. If `attacker` != winner, then winner was defender.
        let mut attacks_faced = 0;
        let mut missed_parry = false;

        for exchange in duel.exchanges() {
            if exchange.attacker != winner {
                attacks_faced += 1;
                if !exchange.result.is_parried() {
                    missed_parry = true;
                }
            }
        }

        if attacks_faced > 0 && !missed_parry {
            achievements.push(Self::SharpTongue);
        }

        // 4. Comeback Kid
        // Replay history to see if winner was ever at match point deficit.
        // We need to know `wins_needed`. It's not directly public on `Duel` via a getter,
        // but we can infer it or iterate until someone won?
        // Actually, `Duel` has `wins_needed` private field.
        // But `result.winner` implies they reached it.
        // Let's assume standard 3 wins or infer from final score?
        // `Duel::with_wins_needed` allows custom.
        // If winner score is X, then wins_needed was X.
        let wins_needed = winner_score;

        let mut current_winner_score = 0;
        let mut current_loser_score = 0;
        let mut faced_match_point = false;

        for exchange in duel.exchanges() {
            // Check condition BEFORE updating scores (state before this exchange)
            if current_loser_score == wins_needed - 1 && current_winner_score < wins_needed {
                faced_match_point = true;
            }

            match exchange.winner {
                w if w == winner => current_winner_score += 1,
                _ => current_loser_score += 1,
            }
        }

        if faced_match_point {
            achievements.push(Self::ComebackKid);
        }

        // 5. Speedrun
        // Won without dropping a single exchange?
        // That's basically Untouchable, but maybe specifically "Minimum exchanges"?
        // Minimum exchanges = wins_needed (if they attack first and opponent fails every time, or they defend and parry and then attack and opponent fails).
        // Actually, to win, you need to score points.
        // You score a point by winning an exchange.
        // So min rounds = wins_needed.
        if duel.exchanges().len() == wins_needed as usize {
            achievements.push(Self::Speedrun);
        }

        achievements
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_untouchable() {
        // Challenger wins 3-0
        let mut duel = Duel::new(); // 3 wins needed
        for _ in 0..3 {
            duel.throw_insult("You fight like a dairy farmer!".to_string())
                .unwrap();
            duel.respond("wrong".to_string()).unwrap(); // Challenger wins exchange
        }

        let badges = Achievement::check(&duel);
        assert!(badges.contains(&Achievement::Untouchable));
        assert!(badges.contains(&Achievement::Speedrun)); // 3 rounds = 3 points
    }

    #[test]
    fn test_comeback_kid() {
        // Wins needed = 3
        // Sequence:
        // 1. Challenger attacks, Defender wins (0-1)
        // 2. Defender attacks, Defender wins (0-2) -> Match Point for Defender
        // 3. Defender attacks, Challenger wins (1-2)
        // 4. Challenger attacks, Challenger wins (2-2)
        // 5. Challenger attacks, Challenger wins (3-2) -> Challenger Wins

        let mut duel = Duel::new();

        // Round 1: Challenger vs Defender (Defender wins)
        duel.throw_insult("You fight like a dairy farmer!".to_string())
            .unwrap();
        duel.respond("How appropriate. You fight like a cow!".to_string())
            .unwrap(); // Defender parries

        // Round 2: Defender attacks (Defender wins)
        duel.throw_insult("You have the manners of a beggar.".to_string())
            .unwrap();
        duel.respond("wrong".to_string()).unwrap(); // Challenger fails, Defender wins (0-2)

        // Round 3: Defender attacks (Challenger wins)
        duel.throw_insult("I've spoken with apes more polite than you!".to_string())
            .unwrap();
        duel.respond("I'm glad to hear you attended your family reunion!".to_string())
            .unwrap(); // Challenger parries (1-2)

        // Round 4: Challenger attacks (Challenger wins)
        duel.throw_insult("Soon you'll be wearing my sword like a shish kebab!".to_string())
            .unwrap();
        duel.respond("wrong".to_string()).unwrap(); // Defender fails (2-2)

        // Round 5: Challenger attacks (Challenger wins)
        duel.throw_insult("People fall at my feet when they see me coming!".to_string())
            .unwrap();
        duel.respond("wrong".to_string()).unwrap(); // Defender fails (3-2)

        assert!(duel.is_finished());
        let badges = Achievement::check(&duel);
        assert!(
            badges.contains(&Achievement::ComebackKid),
            "Should award ComebackKid"
        );
        assert!(
            badges.contains(&Achievement::Encyclopedia),
            "Used 5 insults, so Encyclopedia"
        );
    }

    #[test]
    fn test_sharp_tongue() {
        // Defender wins by parrying everything and never attacking?
        // Wait, if Defender parries, they become Attacker.
        // So:
        // 1. C attacks, D parries (D wins exchange, becomes Attacker).
        // 2. D attacks, C fails (D wins exchange).
        // 3. D attacks, C fails (D wins exchange).
        // D wins 3-0.
        // D faced 1 attack, parried it. Missed 0. -> Sharp Tongue.

        let mut duel = Duel::new();

        // R1: C attacks, D parries.
        duel.throw_insult("You fight like a dairy farmer!".to_string())
            .unwrap();
        duel.respond("How appropriate. You fight like a cow!".to_string())
            .unwrap();

        // R2: D attacks, C fails.
        duel.throw_insult("You have the manners of a beggar.".to_string())
            .unwrap();
        duel.respond("wrong".to_string()).unwrap();

        // R3: D attacks, C fails.
        duel.throw_insult("I've spoken with apes more polite than you!".to_string())
            .unwrap();
        duel.respond("wrong".to_string()).unwrap();

        let badges = Achievement::check(&duel);
        assert!(
            badges.contains(&Achievement::SharpTongue),
            "Parried only attack faced"
        );
        assert!(badges.contains(&Achievement::Untouchable));
    }
}
