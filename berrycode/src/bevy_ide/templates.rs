//! Bevy template types for code generation
//!
//! Generates boilerplate code for common Bevy patterns:
//! Components, Resources, Systems, Plugins, Events, States.

/// Bevy template types for code generation
pub enum BevyTemplate {
    Component {
        name: String,
        fields: Vec<(String, String)>,
    },
    Resource {
        name: String,
        fields: Vec<(String, String)>,
    },
    System {
        name: String,
        params: Vec<String>,
    },
    Plugin {
        name: String,
    },
    StartupSystem {
        name: String,
    },
    Event {
        name: String,
        fields: Vec<(String, String)>,
    },
    State {
        name: String,
        variants: Vec<String>,
    },
}

impl BevyTemplate {
    /// Generate Rust source code for this template
    pub fn generate(&self) -> String {
        match self {
            BevyTemplate::Component { name, fields } => {
                let mut code = format!("#[derive(Component)]\npub struct {} {{\n", name);
                for (field_name, field_type) in fields {
                    code.push_str(&format!("    pub {}: {},\n", field_name, field_type));
                }
                code.push_str("}\n");
                code
            }
            BevyTemplate::Resource { name, fields } => {
                let mut code = format!("#[derive(Resource)]\npub struct {} {{\n", name);
                for (field_name, field_type) in fields {
                    code.push_str(&format!("    pub {}: {},\n", field_name, field_type));
                }
                code.push_str("}\n");
                code
            }
            BevyTemplate::System { name, params } => {
                let param_str = params.join(", ");
                format!(
                    "fn {}({}) {{\n    // TODO: implement system logic\n}}\n",
                    name, param_str
                )
            }
            BevyTemplate::Plugin { name } => {
                format!(
                    "pub struct {};\n\nimpl Plugin for {} {{\n    fn build(&self, app: &mut App) {{\n        app\n            // .add_systems(Startup, setup)\n            // .add_systems(Update, update)\n            ;\n    }}\n}}\n",
                    name, name
                )
            }
            BevyTemplate::StartupSystem { name } => {
                format!(
                    "fn {}(mut commands: Commands) {{\n    // TODO: spawn initial entities\n}}\n",
                    name
                )
            }
            BevyTemplate::Event { name, fields } => {
                let mut code = format!("#[derive(Event)]\npub struct {} {{\n", name);
                for (field_name, field_type) in fields {
                    code.push_str(&format!("    pub {}: {},\n", field_name, field_type));
                }
                code.push_str("}\n");
                code
            }
            BevyTemplate::State { name, variants } => {
                let mut code = format!(
                    "#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]\npub enum {} {{\n",
                    name
                );
                for (i, variant) in variants.iter().enumerate() {
                    if i == 0 {
                        code.push_str(&format!("    #[default]\n    {},\n", variant));
                    } else {
                        code.push_str(&format!("    {},\n", variant));
                    }
                }
                code.push_str("}\n");
                code
            }
        }
    }

    /// Get a human-readable label for the template type
    pub fn type_label(&self) -> &'static str {
        match self {
            BevyTemplate::Component { .. } => "Component",
            BevyTemplate::Resource { .. } => "Resource",
            BevyTemplate::System { .. } => "System",
            BevyTemplate::Plugin { .. } => "Plugin",
            BevyTemplate::StartupSystem { .. } => "Startup System",
            BevyTemplate::Event { .. } => "Event",
            BevyTemplate::State { .. } => "State",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_template() {
        let template = BevyTemplate::Component {
            name: "Health".to_string(),
            fields: vec![
                ("current".to_string(), "f32".to_string()),
                ("max".to_string(), "f32".to_string()),
            ],
        };
        let code = template.generate();
        assert!(code.contains("#[derive(Component)]"));
        assert!(code.contains("pub struct Health"));
        assert!(code.contains("pub current: f32,"));
        assert!(code.contains("pub max: f32,"));
    }

    #[test]
    fn test_plugin_template() {
        let template = BevyTemplate::Plugin {
            name: "GamePlugin".to_string(),
        };
        let code = template.generate();
        assert!(code.contains("pub struct GamePlugin;"));
        assert!(code.contains("impl Plugin for GamePlugin"));
        assert!(code.contains("fn build(&self, app: &mut App)"));
    }

    #[test]
    fn test_state_template() {
        let template = BevyTemplate::State {
            name: "GameState".to_string(),
            variants: vec![
                "Menu".to_string(),
                "Playing".to_string(),
                "Paused".to_string(),
            ],
        };
        let code = template.generate();
        assert!(code.contains("#[derive(States"));
        assert!(code.contains("#[default]"));
        assert!(code.contains("Menu,"));
        assert!(code.contains("Playing,"));
    }
}
