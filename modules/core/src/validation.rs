use serde::{Deserialize, Serialize};

/// Severity of a validation message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationLevel {
    Error,
    Warning,
    Info,
}

/// A single validation finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationMessage {
    pub level: ValidationLevel,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
}

impl ValidationMessage {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level: ValidationLevel::Error,
            code: code.into(),
            message: message.into(),
            target_id: None,
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level: ValidationLevel::Warning,
            code: code.into(),
            message: message.into(),
            target_id: None,
        }
    }

    pub fn info(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level: ValidationLevel::Info,
            code: code.into(),
            message: message.into(),
            target_id: None,
        }
    }

    pub fn with_target(mut self, target_id: impl Into<String>) -> Self {
        self.target_id = Some(target_id.into());
        self
    }
}

/// Aggregated validation output from schema, geometry, or constraint checks.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub messages: Vec<ValidationMessage>,
}

impl ValidationReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, message: ValidationMessage) {
        self.messages.push(message);
    }

    pub fn merge(&mut self, other: ValidationReport) {
        self.messages.extend(other.messages);
    }

    pub fn has_errors(&self) -> bool {
        self.messages
            .iter()
            .any(|m| m.level == ValidationLevel::Error)
    }

    pub fn is_ok(&self) -> bool {
        !self.has_errors()
    }

    pub fn error_count(&self) -> usize {
        self.messages
            .iter()
            .filter(|m| m.level == ValidationLevel::Error)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_tracks_errors() {
        let mut report = ValidationReport::new();
        report.push(ValidationMessage::warning("W001", "minor issue"));
        assert!(report.is_ok());

        report.push(ValidationMessage::error("E001", "blocking issue"));
        assert!(!report.is_ok());
        assert_eq!(report.error_count(), 1);
    }
}
