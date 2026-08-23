# Feature API

`opencad-feature` owns serializable feature definitions and deterministic
regeneration through the kernel-neutral `GeometryKernel` interface.

## Representative multi-feature part

`bearing_carrier()` constructs the checked-in
`examples/bearing_carrier.ocad.d` model. Its nine nodes cover four sketches and
five geometry operations:

1. base sketch and extrusion;
2. circular hub sketch and joined extrusion;
3. central bearing-bore sketch and through cut;
4. bolt-hole tool sketch and extrusion;
5. four-instance circular cut pattern.

Use `opencad_graph::bearing_carrier_parameters()` for its seven explicit-unit
parameters. Call `PartModel::regenerate()` with a `FeatureRegistry` and a
`GeometryKernel`; generated bodies remain disposable outputs of the Design
Graph.

```rust
let mut model = opencad_feature::bearing_carrier()?;
let parameters = opencad_graph::bearing_carrier_parameters();
let registry = opencad_feature::FeatureRegistry::with_defaults();
model.regenerate(&kernel, &registry, Some(&parameters), None)?;
```

The constructor performs no file-system or network I/O. The desktop template
layer owns `.ocad` persistence through the `bearing-carrier` template.
