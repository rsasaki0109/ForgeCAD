use serde::{Deserialize, Serialize};

use opencad_core::{EntityId, Result};

use crate::entity::{LineEntity, SketchEntity};

/// Classification of a detected profile loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileKind {
    Closed,
    Open,
    SelfIntersecting,
}

/// A profile loop formed by connected sketch entities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub entity_ids: Vec<EntityId>,
    pub kind: ProfileKind,
    /// Stable reference for extrude operations (e.g. `sketch:base/profile:outer`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_ref: Option<String>,
}

impl Profile {
    pub fn new(id: impl Into<String>, entity_ids: Vec<EntityId>, kind: ProfileKind) -> Self {
        Self {
            id: id.into(),
            entity_ids,
            kind,
            profile_ref: None,
        }
    }

    pub fn with_ref(mut self, profile_ref: impl Into<String>) -> Self {
        self.profile_ref = Some(profile_ref.into());
        self
    }

    pub fn is_closed(&self) -> bool {
        self.kind == ProfileKind::Closed
    }
}

/// Detect connected loops from lines and circles.
pub fn detect_profiles(entities: &[SketchEntity]) -> Result<Vec<Profile>> {
    let mut profiles = Vec::new();

    for entity in entities {
        if let SketchEntity::Circle(c) = entity {
            if !c.base.construction {
                profiles.push(
                    Profile::new(
                        format!("profile:circle:{}", c.base.id),
                        vec![c.base.id.clone()],
                        ProfileKind::Closed,
                    )
                    .with_ref(format!("profile:circle:{}", c.base.id.as_str())),
                );
            }
        }
    }

    let lines: Vec<&LineEntity> = entities
        .iter()
        .filter_map(|e| match e {
            SketchEntity::Line(l) if !l.base.construction => Some(l),
            _ => None,
        })
        .collect();

    if lines.len() >= 2 {
        let mut used = vec![false; lines.len()];
        for start in 0..lines.len() {
            if used[start] {
                continue;
            }
            if let Some(chain) = trace_chain(start, &lines, &mut used) {
                let kind = classify_chain(&chain, &lines);
                if kind != ProfileKind::Open || chain.len() >= 2 {
                    profiles.push(Profile::new(
                        format!("profile:loop:{}", profiles.len()),
                        chain,
                        kind,
                    ));
                }
            }
        }
    }

    Ok(profiles)
}

fn trace_chain(start: usize, lines: &[&LineEntity], used: &mut [bool]) -> Option<Vec<EntityId>> {
    let start_line = lines[start];
    let mut chain = vec![start_line.base.id.clone()];
    let mut current_end = start_line.end.clone();
    used[start] = true;

    let target_start = start_line.start.clone();
    let max_steps = lines.len() + 1;

    for _ in 0..max_steps {
        if current_end == target_start && chain.len() >= 3 {
            return Some(chain);
        }

        let mut found = false;
        for (i, line) in lines.iter().enumerate() {
            if used[i] {
                continue;
            }
            if line.start == current_end {
                chain.push(line.base.id.clone());
                current_end = line.end.clone();
                used[i] = true;
                found = true;
                break;
            } else if line.end == current_end {
                chain.push(line.base.id.clone());
                current_end = line.start.clone();
                used[i] = true;
                found = true;
                break;
            }
        }

        if !found {
            if chain.len() >= 2 {
                return Some(chain);
            }
            unwind_used(&chain, lines, used);
            return None;
        }
    }

    unwind_used(&chain, lines, used);
    None
}

fn unwind_used(chain: &[EntityId], lines: &[&LineEntity], used: &mut [bool]) {
    for (i, u) in used.iter_mut().enumerate() {
        if chain.iter().any(|id| id == &lines[i].base.id) {
            *u = false;
        }
    }
}

fn classify_chain(entity_ids: &[EntityId], lines: &[&LineEntity]) -> ProfileKind {
    let mut point_visit_count: indexmap::IndexMap<String, usize> = indexmap::IndexMap::new();

    for line_id in entity_ids {
        let Some(line) = lines.iter().find(|l| &l.base.id == line_id) else {
            continue;
        };
        *point_visit_count
            .entry(line.start.as_str().to_string())
            .or_insert(0) += 1;
        *point_visit_count
            .entry(line.end.as_str().to_string())
            .or_insert(0) += 1;
    }

    let start = lines
        .iter()
        .find(|l| entity_ids.first() == Some(&l.base.id))
        .map(|l| l.start.as_str().to_string());

    let end = entity_ids
        .last()
        .and_then(|id| lines.iter().find(|l| &l.base.id == id))
        .map(|l| l.end.as_str().to_string());

    if let (Some(s), Some(e)) = (start, end) {
        if s == e && entity_ids.len() >= 3 {
            if point_visit_count.values().any(|&c| c > 2) {
                return ProfileKind::SelfIntersecting;
            }
            return ProfileKind::Closed;
        }
    }

    if point_visit_count.values().any(|&c| c > 2) {
        return ProfileKind::SelfIntersecting;
    }

    ProfileKind::Open
}

/// Assign stable profile refs for the outermost closed profile in a sketch.
pub fn assign_profile_refs(sketch_id: &str, profiles: &mut [Profile]) {
    let outer = profiles
        .iter()
        .position(|p| p.kind == ProfileKind::Closed)
        .or_else(|| profiles.iter().position(|p| p.is_closed()));

    if let Some(idx) = outer {
        profiles[idx].profile_ref = Some(format!("{sketch_id}/profile:outer"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{CircleEntity, Coord, EntityBase, LineEntity};

    fn ent(id: &str) -> EntityId {
        EntityId::new(id).expect("valid id")
    }

    fn line(id: &str, start: &str, end: &str) -> SketchEntity {
        SketchEntity::Line(LineEntity {
            base: EntityBase {
                id: ent(id),
                construction: false,
            },
            start: ent(start),
            end: ent(end),
        })
    }

    #[test]
    fn detects_rectangular_closed_profile() {
        let entities = vec![
            line("ent:e0", "ent:c0", "ent:c1"),
            line("ent:e1", "ent:c1", "ent:c2"),
            line("ent:e2", "ent:c2", "ent:c3"),
            line("ent:e3", "ent:c3", "ent:c0"),
        ];
        let profiles = detect_profiles(&entities).expect("detect");
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].kind, ProfileKind::Closed);
        assert_eq!(profiles[0].entity_ids.len(), 4);
    }

    #[test]
    fn detects_open_chain() {
        let entities = vec![
            line("ent:e0", "ent:c0", "ent:c1"),
            line("ent:e1", "ent:c1", "ent:c2"),
        ];
        let profiles = detect_profiles(&entities).expect("detect");
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].kind, ProfileKind::Open);
    }

    #[test]
    fn detects_circle_profile() {
        let entities = vec![SketchEntity::Circle(CircleEntity {
            base: EntityBase {
                id: ent("ent:circle_1"),
                construction: false,
            },
            center: ent("ent:pt_center"),
            radius: Coord::literal(5.0),
        })];
        let profiles = detect_profiles(&entities).expect("detect");
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].kind, ProfileKind::Closed);
    }

    #[test]
    fn profile_round_trip() {
        let profile = Profile::new(
            "profile:0",
            vec![ent("ent:e0"), ent("ent:e1")],
            ProfileKind::Closed,
        )
        .with_ref("sketch:base/profile:outer");
        let json = serde_json::to_string(&profile).expect("serialize");
        let restored: Profile = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(profile, restored);
    }

    #[test]
    fn assigns_outer_profile_ref() {
        let mut profiles = vec![
            Profile::new("profile:0", vec![ent("ent:e0")], ProfileKind::Open),
            Profile::new(
                "profile:1",
                vec![ent("ent:e1"), ent("ent:e2")],
                ProfileKind::Closed,
            ),
        ];
        assign_profile_refs("sketch:base", &mut profiles);
        assert_eq!(
            profiles[1].profile_ref.as_deref(),
            Some("sketch:base/profile:outer")
        );
    }
}
