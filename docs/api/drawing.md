# Drawing API

`opencad-drawing` stores drawing sheets as part of the Design Graph and derives
SVG output from referenced model meshes.

## Linear dimensions

`LinearDimension` defines an aligned measurement using explicit meter units:

- `id: DimensionId` (`dim:` prefix)
- `view_id: ViewId`
- `start_model_m` and `end_model_m`: referenced-model coordinates in meters
- `offset_m`: perpendicular annotation offset in sheet meters

`LinearDimension::measured_length_m` derives the 3D measurement. Serialized
dimension labels are intentionally unsupported, so displayed values cannot drift
from the model. `layout_linear_dimension` projects witness points using the
referenced view and returns `DimensionLayout` for renderers.

SVG export formats values in millimeters with two decimal places. Degenerate,
non-finite, missing-view, and projection-overlap cases return `OpenCadError`.

## Hidden lines

`classify_hidden_lines` returns `ClassifiedEdge` values with `LineVisibility`.
One source edge may return multiple ordered values when projected triangle
boundaries or triangle/edge depth crossings change visibility. Adjacent
triangles do not occlude their own edge, and same-face tessellation diagonals
are omitted.

The exported tolerance constants make units and numerical policy explicit:

- `HIDDEN_LINE_DEPTH_TOLERANCE_M = 1e-7 m`
- `HIDDEN_LINE_PROJECTION_TOLERANCE_M = 1e-9 m`
- `HIDDEN_LINE_PARAMETER_TOLERANCE = 1e-9` (dimensionless)
- `HIDDEN_LINE_BARYCENTRIC_TOLERANCE = 1e-9` (dimensionless)

Output order is deterministic by canonical source-edge key and increasing
split parameter. Equal adjacent visibility intervals are merged.
