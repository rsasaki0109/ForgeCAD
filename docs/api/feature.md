# Feature API

`opencad-feature` owns serializable feature definitions and deterministic
regeneration through the kernel-neutral `GeometryKernel` interface.

## Representative multi-feature part

`robot_joint_actuator_housing()` constructs the checked-in
`examples/robot_joint_actuator.ocad.d` model. Its 22 nodes expose nine visible
body milestones:

1. base plate;
2. lower and upper stepped hubs;
3. output-shaft bore and bearing counterbore;
4. eight-hole circular fastener pattern;
5. six-instance circular rib union;
6. mirrored mounting-ear union and mirrored mounting-hole cut.

Use `opencad_graph::robot_joint_housing_parameters()` for its 19 explicit-unit
parameters. Call `PartModel::regenerate()` with a `FeatureRegistry` and a
`GeometryKernel`; generated bodies remain disposable outputs of the Design
Graph.

```rust
let mut model = opencad_feature::robot_joint_actuator_housing()?;
let parameters = opencad_graph::robot_joint_housing_parameters();
let registry = opencad_feature::FeatureRegistry::with_defaults();
let report = model.regenerate(&kernel, &registry, Some(&parameters), None)?;
println!("{}", report.trace.trace_hash_sha256);
```

The constructor performs no file-system or network I/O. The desktop template
layer owns `.ocad` persistence through the `robot-joint` template.
`RegenReport.trace` records deterministic execution evidence; see
[Change impact and regeneration trace](change-impact-and-regeneration-trace.md).

## Feature-build animation

`opencad animate-features` regenerates a part, omits standalone pattern-tool
bodies, and renders the remaining body-producing milestones in deterministic
Feature Graph order. Every frame uses a camera fitted to the final body so
geometry growth is directly comparable:

```bash
opencad animate-features examples/robot_joint_actuator.ocad.d build.gif \
  --frames 54 --fps 9 --orbit-deg 35 --pitch-deg 30
```
