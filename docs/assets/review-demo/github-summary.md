## MusubiCAD Design Review

**Status:** ✅ All 2 expected effects passed

| Context | Value |
|---|---|
| Document | doc:bearing_carrier_001 |
| Intent | Increase the bearing hub engagement depth |
| Rationale | The revised bearing stack requires 6 mm more axial support without changing the bore or bolt circle. |
| Patch | review_bearing_carrier_patch.json |

### Semantic changes

| Change | Before | After |
|---|---|---|
| Parameter param:boss_height | 14 mm | 20 mm |
| Mass | 131.02 g | 145.18 g |

### Regenerated geometry

| Property | Before | After |
|---|---:|---:|
| Volume | 48.53 cm³ | 53.77 cm³ |
| Mass | 131.02 g | 145.18 g |
| Bounds | 95.86 × 70.27 × 14.00 mm | 95.86 × 70.27 × 20.00 mm |
| Triangles (count) | 732 | 732 |

### Expected effects

| Status | Expectation | Evidence |
|---|---|---|
| ✅ | Parameter param:boss_height equals 20 mm | parameter 'param:boss_height' expression is 20 mm |
| ✅ | Mass delta is between 0.014 kg and 0.019 kg | mass delta is 0.014158874927520326 kg |

The workflow artifact contains `review.html`, `review.json`, `comparison.gif`, and the before/after images.
