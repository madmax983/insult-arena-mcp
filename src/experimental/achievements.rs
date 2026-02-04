use serde::{Deserialize, Serialize};

use crate::{Duel, Duelist};

/// An award earned for specific feats in a duel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Achievement {
    /// Unique identifier for the achievement.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Description of how it was earned.
    pub description: String,
    /// Emoji icon.
    pub icon: String,
}

impl Achievement {
    fn new(id: &str, name: &str, desc: &str, icon: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            icon: icon.to_string(),
        }
    }
}

/// Analyzes a completed duel and returns a list of earned achievements.
#[must_use]
pub fn analyze(duel: &Duel) -> Vec<Achievement> {
    let mut achievements = Vec::new();
    let Some(result) = duel.result() else {
        return achievements;
    };

    let winner = result.winner;
    let (winner_score, loser_score) = match winner {
        Duelist::Challenger => (result.challenger_score, result.defender_score),
        Duelist::Defender => (result.defender_score, result.challenger_score),
    };

    // 1. Untouchable: Winner took 0 damage
    if loser_score == 0 {
        achievements.push(Achievement::new(
            "untouchable",
            "Untouchable",
            "Won the duel without losing a single point.",
            "🛡️",
        ));
    }

    // 2. Clutch Master: Won with 1 point margin (in a standard 3-point game)
    if winner_score >= 3 && loser_score == winner_score - 1 {
        achievements.push(Achievement::new(
            "clutch_master",
            "Clutch Master",
            "Won a nail-biter by a single point.",
            "🤏",
        ));
    }

    // 3. Comeback Kid: Was trailing by 2 or more points at any time
    let mut c_score = 0;
    let mut d_score = 0;
    let mut max_deficit = 0;

    for exchange in &result.exchanges {
        // Check deficit BEFORE updating score for this round
        let (my_score, opp_score) = match winner {
            Duelist::Challenger => (c_score, d_score),
            Duelist::Defender => (d_score, c_score),
        };

        let deficit = opp_score - my_score;
        if deficit > max_deficit {
            max_deficit = deficit;
        }

        match exchange.winner {
            Duelist::Challenger => c_score += 1,
            Duelist::Defender => d_score += 1,
        }
    }

    if max_deficit >= 2 {
        achievements.push(Achievement::new(
            "comeback_kid",
            "Comeback Kid",
            "Rallied to win after trailing by 2 or more points.",
            "🔄",
        ));
    }

    // 4. Chatterbox: Long duel (> 10 rounds)
    if result.exchanges.len() > 10 {
        achievements.push(Achievement::new(
            "chatterbox",
            "Chatterbox",
            "Duel lasted over 10 rounds.",
            "🗣️",
        ));
    }

    achievements
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::Duel;

    #[test]
    fn test_untouchable() {
        // Win 3-0
        let mut duel = Duel::new();
        // C wins 1
        duel.throw_insult("You fight like a dairy farmer!".to_string())
            .unwrap();
        duel.respond("wrong".to_string()).unwrap();
        // C wins 2
        duel.throw_insult("You fight like a dairy farmer!".to_string())
            .unwrap();
        duel.respond("wrong".to_string()).unwrap();
        // C wins 3
        duel.throw_insult("You fight like a dairy farmer!".to_string())
            .unwrap();
        duel.respond("wrong".to_string()).unwrap();

        let achievements = analyze(&duel);
        assert!(achievements.iter().any(|a| a.id == "untouchable"));
        assert!(!achievements.iter().any(|a| a.id == "clutch_master"));
    }

    #[test]
    fn test_clutch_master() {
        // Win 3-2
        let mut duel = Duel::new();

        // C wins 1 (1-0)
        duel.throw_insult("You fight like a dairy farmer!".to_string())
            .unwrap();
        duel.respond("wrong".to_string()).unwrap();

        // D wins 1 (1-1)
        duel.throw_insult("You fight like a dairy farmer!".to_string())
            .unwrap();
        duel.respond("How appropriate. You fight like a cow!".to_string())
            .unwrap();

        // D wins 2 (1-2)
        // Defender is attacking now
        duel.throw_insult("You have the manners of a beggar.".to_string())
            .unwrap();
        // Challenger fails to respond
        duel.respond("wrong".to_string()).unwrap();

        // Now D is attacking again.
        // C wins 2 (2-2)
        duel.throw_insult("You have the manners of a beggar.".to_string())
            .unwrap();
        // Challenger responds correctly
        duel.respond("I wanted to make sure you'd feel comfortable with me.".to_string())
            .unwrap();

        // Now C is attacking.
        // C wins 3 (3-2)
        duel.throw_insult("You have the manners of a beggar.".to_string())
            .unwrap();
        duel.respond("wrong".to_string()).unwrap();

        let achievements = analyze(&duel);
        assert!(!achievements.iter().any(|a| a.id == "untouchable"));
        assert!(achievements.iter().any(|a| a.id == "clutch_master"));
    }

    #[test]
    fn test_comeback_kid() {
        // Win 3-2 after being down 0-2
        let mut duel = Duel::new();

        // D wins 1 (0-1) - C attacks, D responds
        duel.throw_insult("You fight like a dairy farmer!".to_string())
            .unwrap();
        duel.respond("How appropriate. You fight like a cow!".to_string())
            .unwrap();

        // D wins 2 (0-2) - D attacks, C fails
        duel.throw_insult("You have the manners of a beggar.".to_string())
            .unwrap();
        duel.respond("wrong".to_string()).unwrap();

        // Now score is 0-2 (Defender leads). C needs to win 3 in a row.
        // D attacks, C responds (1-2)
        duel.throw_insult("You have the manners of a beggar.".to_string())
            .unwrap();
        duel.respond("I wanted to make sure you'd feel comfortable with me.".to_string())
            .unwrap();

        // C attacks, D fails (2-2)
        duel.throw_insult("You fight like a dairy farmer!".to_string())
            .unwrap();
        duel.respond("wrong".to_string()).unwrap();

        // C attacks, D fails (3-2)
        duel.throw_insult("You fight like a dairy farmer!".to_string())
            .unwrap();
        duel.respond("wrong".to_string()).unwrap();

        let achievements = analyze(&duel);
        assert!(achievements.iter().any(|a| a.id == "comeback_kid"));
        assert!(achievements.iter().any(|a| a.id == "clutch_master")); // 3-2 is also clutch
    }

    #[test]
    fn test_chatterbox() {
        // With wins_needed=6, we can reach 11 rounds (5-6 or 6-5).
        let mut duel = Duel::with_wins_needed(6);

        // 1. C wins 5 times. (Score 5-0)
        for _ in 0..5 {
            duel.throw_insult("You fight like a dairy farmer!".to_string())
                .unwrap();
            duel.respond("wrong".to_string()).unwrap();
        }

        // 2. D wins 5 times. (Score 5-5)
        // C is currently attacker.
        // Exchange 6: C attacks, D parries. (Score 5-1). D becomes attacker.
        duel.throw_insult("You fight like a dairy farmer!".to_string())
            .unwrap();
        duel.respond("How appropriate. You fight like a cow!".to_string())
            .unwrap();

        // Exchange 7,8,9,10: D attacks, C fails.
        for _ in 0..4 {
            duel.throw_insult("You fight like a dairy farmer!".to_string())
                .unwrap();
            duel.respond("wrong".to_string()).unwrap();
        }

        // Score is now 5-5. Rounds played: 5 + 1 + 4 = 10.
        // Next round (11th) will finish it.
        // D is attacker. D attacks, C fails. D wins (5-6).
        duel.throw_insult("You fight like a dairy farmer!".to_string())
            .unwrap();
        duel.respond("wrong".to_string()).unwrap();

        assert!(duel.is_finished());
        assert_eq!(duel.exchanges().len(), 11);

        let achievements = analyze(&duel);
        assert!(achievements.iter().any(|a| a.id == "chatterbox"));
    }
}
