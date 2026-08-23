# MCAD-P5-005 golden evidence example

The representative bracket workflow is described by
[`fixtures/golden/mcad_p5_005_end_to_end.json`](../fixtures/golden/mcad_p5_005_end_to_end.json).
It connects:

- `examples/bracket.ocad.d` and the `bracket_with_hole` mass fixture;
- `examples/assembly_two_brackets.ocad.d` and its placed OCCT scene;
- the partial-occlusion drawing SVG;
- `examples/agent/review_width_patch.json` and its Agent API dry run;
- `examples/bearing_carrier.ocad.d`, its hub-height review DesignPatch, and the
  CLI review artifacts under `docs/assets/review-demo`;
- the Desktop preview result for the bracket fixture.

Run the deterministic cross-artifact check from the repository root:

```text
cargo test -p opencad-cli mcad_p5_005_end_to_end_artifacts_are_deterministic
```

The test uses explicit SI units and tolerances for mass, volume, and bounds,
resolves semantic topology references against the current regenerated body,
and requires the JSON, HTML, Markdown, and SVG outputs to remain stable.
