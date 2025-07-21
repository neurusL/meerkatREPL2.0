use std::collections::HashMap;

/// Keeps track of the version history of all variables/definitions
#[derive(Debug, Default)]
pub struct VersionMap {
    map: HashMap<String, usize>,
}

impl VersionMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Get current versioned name like "x0", "a1"
    pub fn get_latest(&self, base: &str) -> String {
        match self.map.get(base) {
            Some(v) => format!("{base}{v}"),
            None => format!("{base}0"),
        }
    }

    /// Increment version and return new name
    pub fn next_version(&mut self, base: &str) -> String {
        let entry = self.map.entry(base.to_string()).or_insert(0);
        *entry += 1;
        format!("{base}{entry}")
    }

    /// Reset all versions
    pub fn reset(&mut self) {
        self.map.clear();
    }
}
