use crate::error::{AppError, Result};

#[derive(Debug, Clone, Default)]
pub struct Policy {
    pub read_only: bool,
    pub allow_destructive: bool,
}

impl Policy {
    pub fn from_env() -> Self {
        Self {
            read_only: env_flag("AGENT_ANDROID_READ_ONLY"),
            allow_destructive: env_flag("AGENT_ANDROID_ALLOW_DESTRUCTIVE"),
        }
    }

    pub fn ensure_write(&self, action: &str) -> Result<()> {
        if self.read_only {
            return Err(AppError::new(
                "PERM_DENIED",
                format!("{action} is blocked because read-only mode is enabled"),
            )
            .with_suggestion("Set AGENT_ANDROID_READ_ONLY=0 to allow actions"));
        }
        Ok(())
    }

    pub fn ensure_destructive(&self, action: &str, confirm: bool) -> Result<()> {
        self.ensure_write(action)?;
        if !confirm && !self.allow_destructive {
            return Err(AppError::new(
                "CONFIRMATION_REQUIRED",
                format!("{action} is destructive and was not confirmed"),
            )
            .with_suggestion(
                "Pass confirm=true, or set AGENT_ANDROID_ALLOW_DESTRUCTIVE=1 to allow it",
            ));
        }
        Ok(())
    }
}

fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}
