# Cross-artifact golden evidence

The P5-005 manifest is the reviewable contract for representative engineering
evidence, rather than a public model API:

[`fixtures/golden/mcad_p5_005_end_to_end.json`](../../fixtures/golden/mcad_p5_005_end_to_end.json)

It associates a part fixture, assembly fixture, drawing SVG, CLI review input,
Desktop preview evidence, and an Agent `DesignPatch` dry run. Unit-bearing names and adjacent tolerances make the
expected values explicit, including the assembly interference count. The CLI test compares exact serialized review
artifacts and exact SVG text, while geometry and mass values use their declared
tolerances. Topology entries distinguish stable semantic identity (`ref_id`,
kind, producer, and role) from current kernel references, which are resolved
against the regenerated body rather than persisted kernel IDs.

This contract does not add a serialized `.ocad` field or change any public
runtime API. The test calls `opencad_desktop::preview_document` and
`AgentApi::patch_dry_run` directly, so those surfaces cannot drift behind a
fixture-only assertion. When a public geometry, drawing, assembly, or review result
changes, update the manifest and its focused test together with the relevant
API/architecture documentation.
