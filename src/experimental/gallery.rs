use rust_mcp_sdk::schema::{Tool, ToolInputSchema};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, VecDeque};

/// Actions a spectator can take.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpectatorAction {
    /// Cheers for a duelist.
    Cheer {
        /// The name of the spectator (session ID).
        spectator: String,
        /// The target of the cheer (Challenger/Defender).
        target: String,
    },
    /// Boos a duelist.
    Boo {
        /// The name of the spectator.
        spectator: String,
        /// The target of the boo.
        target: String,
    },
    /// Heckles with a message.
    Heckle {
        /// The name of the spectator.
        spectator: String,
        /// The heckle message.
        message: String,
    },
}

/// Manages spectator interactions.
#[derive(Debug, Default)]
pub struct Gallery {
    /// History of spectator actions (for potential replay/logs).
    /// Kept as a rolling buffer to prevent memory leaks.
    history: VecDeque<SpectatorAction>,
}

const MAX_HISTORY_SIZE: usize = 100;

impl Gallery {
    /// Creates a new Gallery.
    #[must_use]
    pub fn new() -> Self {
        Self {
            history: VecDeque::with_capacity(MAX_HISTORY_SIZE),
        }
    }

    fn push_action(&mut self, action: SpectatorAction) {
        if self.history.len() >= MAX_HISTORY_SIZE {
            self.history.pop_front();
        }
        self.history.push_back(action);
    }

    /// Handles a cheer action.
    pub fn cheer(&mut self, spectator: String, target: String) -> String {
        let action = SpectatorAction::Cheer {
            spectator: spectator.clone(),
            target: target.clone(),
        };
        self.push_action(action);
        format!("🎉 Spectator {} cheers for {}!", spectator, target)
    }

    /// Handles a boo action.
    pub fn boo(&mut self, spectator: String, target: String) -> String {
        let action = SpectatorAction::Boo {
            spectator: spectator.clone(),
            target: target.clone(),
        };
        self.push_action(action);
        format!("🍅 Spectator {} boos {}!", spectator, target)
    }

    /// Handles a heckle action.
    pub fn heckle(&mut self, spectator: String, message: String) -> String {
        // Sanitize message?
        // Assume input validation happens before calling this.
        let action = SpectatorAction::Heckle {
            spectator: spectator.clone(),
            message: message.clone(),
        };
        self.push_action(action);
        format!("📢 Spectator {} shouts: \"{}\"", spectator, message)
    }
}

// --- Tool Definitions ---

fn string_param_schema(name: &str, description: &str) -> ToolInputSchema {
    let mut props = HashMap::new();
    let mut prop_map = serde_json::Map::new();
    prop_map.insert("type".to_string(), json!("string"));
    prop_map.insert("description".to_string(), json!(description));
    props.insert(name.to_string(), prop_map);

    ToolInputSchema::new(vec![name.to_string()], Some(props), None)
}

/// Tool: `cheer`
pub fn tool_cheer() -> Tool {
    Tool {
        name: "cheer".to_string(),
        description: Some("Cheer for a duelist (Challenger or Defender) to boost their morale!".to_string()),
        input_schema: string_param_schema("target", "Who to cheer for (e.g., 'Challenger', 'Defender')"),
        annotations: None,
        execution: None,
        icons: vec![],
        meta: None,
        output_schema: None,
        title: None,
    }
}

/// Tool: `boo`
pub fn tool_boo() -> Tool {
    Tool {
        name: "boo".to_string(),
        description: Some("Boo a duelist to lower their morale! Throw a virtual tomato!".to_string()),
        input_schema: string_param_schema("target", "Who to boo (e.g., 'Challenger', 'Defender')"),
        annotations: None,
        execution: None,
        icons: vec![],
        meta: None,
        output_schema: None,
        title: None,
    }
}

/// Tool: `heckle`
pub fn tool_heckle() -> Tool {
    Tool {
        name: "heckle".to_string(),
        description: Some("Shout a heckle from the audience to distract the duelists!".to_string()),
        input_schema: string_param_schema("message", "Your heckle message"),
        annotations: None,
        execution: None,
        icons: vec![],
        meta: None,
        output_schema: None,
        title: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cheer() {
        let mut gallery = Gallery::new();
        let msg = gallery.cheer("User1".into(), "Challenger".into());
        assert_eq!(msg, "🎉 Spectator User1 cheers for Challenger!");
        assert_eq!(gallery.history.len(), 1);
    }

    #[test]
    fn test_boo() {
        let mut gallery = Gallery::new();
        let msg = gallery.boo("User2".into(), "Defender".into());
        assert_eq!(msg, "🍅 Spectator User2 boos Defender!");
        assert_eq!(gallery.history.len(), 1);
    }

    #[test]
    fn test_heckle() {
        let mut gallery = Gallery::new();
        let msg = gallery.heckle("User3".into(), "You fight like a cow!".into());
        assert_eq!(msg, "📢 Spectator User3 shouts: \"You fight like a cow!\"");
        assert_eq!(gallery.history.len(), 1);
    }
}
