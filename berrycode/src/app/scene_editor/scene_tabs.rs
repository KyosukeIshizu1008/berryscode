//! Multiple scene tab management.
//!
//! Each [`SceneTab`] holds a snapshot of a [`SceneModel`] together with a
//! human-readable label (typically the file stem). The app keeps a `Vec<SceneTab>`
//! and an `active_scene_tab` index; switching tabs swaps the active `scene_model`.

use super::model::SceneModel;

#[derive(Debug, Clone)]
pub struct SceneTab {
    pub model: SceneModel,
    pub label: String,
}

impl SceneTab {
    pub fn new(model: SceneModel, label: String) -> Self {
        Self { model, label }
    }
}
