//! Process-wide state shared by Tauri commands.

use chrono::{DateTime, Utc};

pub struct AppState {
    started_at: DateTime<Utc>,
}

impl AppState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            started_at: Utc::now(),
        }
    }

    #[must_use]
    pub const fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
