# README Preview Assets

The orbit GIF and PNG screenshots in the repository root README are produced by the headless renderer and CLI export commands, then assembled with `ffmpeg`.

## Headless renderer

`opencad-render` (`modules/render`) provides `OffscreenRenderer`, which draws offscreen to an `Rgba8UnormSrgb` color target—the same sRGB surface format as the interactive viewport.

Lighting and backdrop are defined in `modules/render/src/solid.rs`:

- **Solids:** studio three-point lighting (key, fill, rim) on a brushed-steel base.
- **Background:** a vertical gradient backdrop drawn before the mesh passes.
- **Feature edges:** boundary edges and sharp creases (`modules/render/src/edges.rs`) are drawn over the shaded solid in graphite, so holes, bosses, fillets, and chamfers read as a CAD outline at any camera angle. The crease threshold defaults to 25°.

## CLI commands

### `opencad screenshot`

```bash
opencad screenshot <input> <output.png>
```

Renders a single **512×512** PNG of the active body in the given `.ocad` document (directory or file). Uses the document’s default camera unless overridden by other tooling.

### `opencad turntable`

```bash
opencad turntable <input> <out_dir> [--frames N] [--width W] [--height H] [--pitch DEG] [--overlay]
```

Renders a full **360° orbit** as a numbered PNG sequence:

`frame_0000.png`, `frame_0001.png`, …

| Option | Default | Description |
|---|---|---|
| `--frames` | `48` | Number of frames around one revolution |
| `--width` | `1600` | Frame width in pixels |
| `--height` | `900` | Frame height in pixels |
| `--pitch` | `28` | Camera pitch in degrees |
| `--overlay` | off | Include sketch overlay when the document has one |

Render at high resolution, then downscale with `ffmpeg` using the **lanczos** filter for supersampling anti-aliasing in the final GIF or video.

## Automated regeneration

[`docs/assets/generate.sh`](../assets/generate.sh) regenerates the committed README assets from the example documents under `examples/` (for example `examples/bracket.ocad.d`):

- `preview.gif` — a 360° turntable orbit of the bracket.
- `preview_param.gif` — the `width 80 mm → 100 mm` parametric patch morph.
- `preview.png` and the `preview_pin_*.png` pattern stills.

The script renders frames with `opencad turntable` (using `--frames 1` for stills), then uses `ffmpeg` **`palettegen`** and **`paletteuse`** to build a clean GIF palette so the gradient backdrop does not dither into noise.

## Manual workflow

Typical steps for a turntable orbit GIF:

```bash
cargo run -p opencad-cli -- turntable examples/bracket.ocad.d /tmp/frames --frames 48
```

Assemble the frame sequence with `ffmpeg` palette filters:

```bash
ffmpeg -i /tmp/frames/frame_%04d.png -vf "fps=24,scale=1280:-1:flags=lanczos,palettegen" /tmp/palette.png
ffmpeg -i /tmp/frames/frame_%04d.png -i /tmp/palette.png -lavfi "fps=24,scale=1280:-1:flags=lanczos [x]; [x][1:v] paletteuse" preview.gif
```

Adjust `fps`, output size, and frame count to taste.

## See also

- [`docs/assets/generate.sh`](../assets/generate.sh) — one-shot regeneration of committed README assets
- [`README.md`](../../README.md) — where the preview GIF and PNGs are displayed
