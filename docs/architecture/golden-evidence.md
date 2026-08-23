# Deterministic engineering evidence

MCAD-P5-005 keeps the engineering evidence shown by the part, assembly,
drawing, CLI review, desktop renderer, and Agent patch paths in one checked-in
manifest:

[`fixtures/golden/mcad_p5_005_end_to_end.json`](../../fixtures/golden/mcad_p5_005_end_to_end.json)

The manifest is a fixture contract, not a second model source. The Design Graph
and the expanded `.ocad.d` documents remain authoritative; the manifest records
the expected regeneration results and artifact paths. Numeric fields carry
their units in their names (`_m`, `_m3`, `_kg`, and `kg_per_m3`) and every
floating-point comparison uses the tolerance recorded next to the value.

The end-to-end test lives with the CLI review generator so it can exercise the
same user-facing path. It performs these deterministic checks:

- OCCT regeneration of `examples/bracket.ocad.d` fixes mass, density, bounding
  box, semantic TopoRef identity, and current face/edge resolution.
- OCCT regeneration of `examples/assembly_two_brackets.ocad.d` fixes instance
  counts, strict-tolerance interference count, mass, and the placed-scene
  bounding box.
- The drawing renderer is compared byte-for-byte with the partial-occlusion
  SVG golden, including visible and hidden segment counts.
- The Agent `DesignPatch` runs through `AgentApi::patch_dry_run`, and the
  Desktop path runs through `preview_document` with golden triangle/model counts
  and tolerance-compared bounds.
- The CLI review geometry and two independent review runs are linked to the
  same fixture. `review.json`, `review.html`, and
  `github-summary.md` must match both each other across runs and the checked-in
  `docs/assets/review-demo` artifacts.

Run the focused contract with:

```text
cargo test -p opencad-cli mcad_p5_005_end_to_end_artifacts_are_deterministic
```

Review image files are generated in temporary directories during the test;
only the text artifacts and SVG required for deterministic review are golden.
