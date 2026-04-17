//! Bevy States editor: design game state transitions.
//!
//! Lets users define app states (Menu, Playing, Paused, GameOver) and
//! transitions between them. Generates the Bevy States enum + OnEnter/OnExit
//! system registration code.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub name: String,
    pub position: [f32; 2], // node graph position
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: usize,
    pub to: usize,
    pub condition: String, // human-readable condition description
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StateGraph {
    pub states: Vec<GameState>,
    pub transitions: Vec<StateTransition>,
    pub initial_state: usize,
}

impl StateGraph {
    pub fn default_game_states() -> Self {
        Self {
            states: vec![
                GameState {
                    name: "Menu".into(),
                    position: [100.0, 150.0],
                },
                GameState {
                    name: "Playing".into(),
                    position: [300.0, 150.0],
                },
                GameState {
                    name: "Paused".into(),
                    position: [300.0, 300.0],
                },
                GameState {
                    name: "GameOver".into(),
                    position: [500.0, 150.0],
                },
            ],
            transitions: vec![
                StateTransition {
                    from: 0,
                    to: 1,
                    condition: "Start Game".into(),
                },
                StateTransition {
                    from: 1,
                    to: 2,
                    condition: "Pause".into(),
                },
                StateTransition {
                    from: 2,
                    to: 1,
                    condition: "Resume".into(),
                },
                StateTransition {
                    from: 1,
                    to: 3,
                    condition: "Player Dies".into(),
                },
                StateTransition {
                    from: 3,
                    to: 0,
                    condition: "Restart".into(),
                },
            ],
            initial_state: 0,
        }
    }
}

/// Generate Bevy States enum code
pub fn generate_states_code(graph: &StateGraph) -> String {
    let mut code = String::new();
    code.push_str("use bevy::prelude::*;\n\n");
    code.push_str(
        "#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]\n",
    );
    code.push_str("pub enum GameState {\n");
    for (i, state) in graph.states.iter().enumerate() {
        if i == graph.initial_state {
            code.push_str("    #[default]\n");
        }
        code.push_str(&format!("    {},\n", state.name));
    }
    code.push_str("}\n");
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_4_states() {
        let g = StateGraph::default_game_states();
        assert_eq!(g.states.len(), 4);
        assert_eq!(g.transitions.len(), 5);
    }

    #[test]
    fn generate_code_has_derive_states() {
        let g = StateGraph::default_game_states();
        let code = generate_states_code(&g);
        assert!(code.contains("#[derive("));
        assert!(code.contains("States)]"));
        assert!(code.contains("Menu"));
        assert!(code.contains("#[default]"));
    }
}
