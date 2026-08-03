

use std::collections::HashMap;

pub struct SystemActionRegistry {
    actions: HashMap<&'static str, fn(&mut App, String) -> Result<(), String>>,
}

impl SystemActionRegistry {
    pub fn new() -> Self {
        Self {
            actions: HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        id: &'static str,
        action: fn(&mut App, String) -> Result<(), String>,
    ) {
        self.actions.insert(id, action);
    }

    pub fn execute(
        &self,
        id: &str,
        app: &mut App,
        value: String,
    ) -> Result<(), String> {
        let action = self.actions
            .get(id)
            .ok_or_else(|| format!("Unknown system action {}", id))?;

        action(app, value)
    }
}