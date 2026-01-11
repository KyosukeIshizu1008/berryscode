//! Analytics module for aider

use crate::berrycode::Result;

pub struct Analytics {
    enabled: bool,
}

impl Analytics {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub fn event(&self, _event_name: &str, _properties: serde_json::Value) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        // TODO: Implement analytics tracking
        Ok(())
    }
}
