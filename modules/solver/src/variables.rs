use std::collections::HashMap;

/// Index into the variable vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VarId(pub u32);

impl VarId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Mutable numeric variable store used by the solver.
#[derive(Debug, Clone, PartialEq)]
pub struct VarSet {
    values: Vec<f64>,
}

impl VarSet {
    pub fn new(values: Vec<f64>) -> Self {
        Self { values }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn get(&self, id: VarId) -> f64 {
        self.values[id.index()]
    }

    pub fn set(&mut self, id: VarId, value: f64) {
        self.values[id.index()] = value;
    }

    pub fn values(&self) -> &[f64] {
        &self.values
    }

    pub fn values_mut(&mut self) -> &mut [f64] {
        &mut self.values
    }
}

/// Maps semantic keys (e.g. point entity id + axis) to solver variables.
#[derive(Debug, Default)]
pub struct VariableRegistry {
    next_id: u32,
    keys: HashMap<String, VarId>,
}

impl VariableRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, key: impl Into<String>) -> VarId {
        let key = key.into();
        if let Some(id) = self.keys.get(&key) {
            return *id;
        }
        let id = VarId(self.next_id);
        self.next_id += 1;
        self.keys.insert(key, id);
        id
    }

    pub fn get(&self, key: &str) -> Option<VarId> {
        self.keys.get(key).copied()
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn initial_values(&self) -> Vec<f64> {
        vec![0.0; self.len()]
    }
}

/// Convenience helpers for 2D point variables.
pub fn point_x(registry: &mut VariableRegistry, point_id: &str) -> VarId {
    registry.register(format!("{point_id}.x"))
}

pub fn point_y(registry: &mut VariableRegistry, point_id: &str) -> VarId {
    registry.register(format!("{point_id}.y"))
}

pub fn radius_var(registry: &mut VariableRegistry, circle_id: &str) -> VarId {
    registry.register(format!("{circle_id}.radius"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_assigns_unique_ids() {
        let mut reg = VariableRegistry::new();
        let x = point_x(&mut reg, "ent:pt_1");
        let y = point_y(&mut reg, "ent:pt_1");
        assert_ne!(x, y);
        assert_eq!(point_x(&mut reg, "ent:pt_1"), x);
    }
}
