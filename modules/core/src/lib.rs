//! OpenCAD core types: stable IDs, units, errors, document metadata, and
//! transaction primitives.

pub mod document;
pub mod error;
pub mod id;
pub mod manifest;
pub mod serialize;
pub mod transaction;
pub mod units;
pub mod validation;

pub use document::DocumentMetadata;
pub use error::{OpenCadError, Result};
pub use id::{
    BodyId, ComponentId, ConstraintId, DocumentId, EntityId, FeatureId, MaterialId, ParameterId,
    PatchId, SketchId, TopoRefId,
};
pub use manifest::OcadManifest;
pub use serialize::{sha256_hex, sorted_map, to_pretty_json};
pub use transaction::{Transaction, TransactionAction, TransactionLog};
pub use units::{Angle, Density, DensityUnit, Expression, Length, LengthUnit, Mass, MassUnit};
pub use validation::{ValidationLevel, ValidationMessage, ValidationReport};
