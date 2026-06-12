use serde::{Deserialize, Serialize};

use opencad_core::{ConstraintId, Expression};

use crate::constraint::DistanceTarget;

/// UI-facing dimension annotation linked to an underlying constraint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dimension {
    pub id: ConstraintId,
    pub name: String,
    pub target: DistanceTarget,
    pub expr: Expression,
    #[serde(default)]
    pub driving: bool,
}

impl Dimension {
    pub fn new(
        id: ConstraintId,
        name: impl Into<String>,
        target: DistanceTarget,
        expr: Expression,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            target,
            expr,
            driving: true,
        }
    }
}
