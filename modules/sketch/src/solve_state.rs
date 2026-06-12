use serde::{Deserialize, Serialize};

/// Result of the sketch constraint solver.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SolveState {
    #[default]
    Unknown,
    UnderConstrained {
        dof: i32,
    },
    FullyConstrained,
    OverConstrained {
        redundant: usize,
    },
    Failed {
        message: String,
    },
}

impl SolveState {
    pub fn is_solved(&self) -> bool {
        matches!(self, Self::FullyConstrained)
    }
}
