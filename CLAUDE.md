# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A Rust web app: point a camera at a hand-drawn diagram (boxes, circles,
diamonds, arrows, labels — on paper or a whiteboard) and it live-detects the
shapes/text and turns them into [D2](https://d2lang.com) diagram-as-code,
plus a rendered preview image. The entire pipeline — camera capture, CV
shape detection, OCR, D2 codegen, and D2→SVG rendering — runs client-side in
WASM; no frame is ever uploaded to a server.

**Current status**: early scaffold (M0 done, M1 and M3 started). `vision`
classifies contours into rectangle/circle/diamond shapes with noise
filtering (tested natively, including noisy synthetic scenes) and `d2gen`
turns a `Diagram` into D2 text; both are unit-tested but **not wired into
`web` yet** — `web` requests camera access and shows the live feed in a
`<video>` element, but nothing calls `vision`/`d2gen` on its frames, so the
app produces no diagram output yet. Canvas frame grabbing, arrow/connector
detection, OCR, and D2 rendering are not implemented. See "Milestones" below.

## Commands

```sh
# Run the pure-Rust pipeline tests (vision + d2gen) — no browser/wasm needed,
# this is the fast inner dev loop for the CV/D2 algorithm work.
cargo test --workspace

# Format / lint (CI enforces both)
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings

# Build the WASM frontend (must be run from crates/web; Trunk looks for
# index.html in the current directory)
cd crates/web && trunk build
cd crates/web && trunk serve   # dev server with live reload, no Axum needed

# Run a single test
cargo test -p vision detects_correct_shapes_in_a_noisy_realistic_scene
cargo test -p d2gen renders_shapes_and_a_directed_labeled_edge

# Serve the production build via the Rust server (run from repo root, after
# a `trunk build` — the server reads crates/web/dist as a relative path)
cargo run -p server
```

## Architecture

Cargo workspace with four crates, each with one job:

```
crates/
  vision/   # pure Rust, no wasm/web deps — CV pipeline, tested natively
  d2gen/    # pure Rust — Diagram -> D2 source text, tested natively
  web/      # Leptos WASM frontend (Trunk-built) — the ONLY crate touching
            # web-sys/wasm-bindgen/the DOM
  server/   # thin Axum server — just serves crates/web/dist, no business logic
```

The split matters: `vision` and `d2gen` are plain Rust with plain
`cargo test` — the CV/codegen algorithm work does not require a browser or
wasm toolchain to iterate on. Only `web` deals with the DOM, camera APIs, and
JS interop.

### `vision` — the CV pipeline

Takes a raw RGBA/grayscale frame (handed in by `web` from a canvas
`ImageData` — `vision` itself never touches the DOM) and produces a
`Diagram { nodes, edges }` (see `crates/vision/src/model.rs`):

1. Grayscale + binarize (`imageproc::contrast::threshold`).
2. Contour extraction (`imageproc::contours::find_contours`).
3. **Implemented**: shape classification in
   `crates/vision/src/pipeline.rs::classify_contour`, by *solidity* (contour
   polygon area via the shoelace formula, divided by bounding-box area) —
   analytically well-separated bands for the three recognized shapes when
   filled: rectangle (axis-aligned) ~1.0, circle ~π/4 ≈ 0.785, diamond
   (bbox-inscribed rhombus) ~0.5. A minimum-area threshold drops speckle/dust
   noise; a maximum-area-fraction-of-frame threshold drops a traced
   background/border. Contours outside all bands or size bounds are discarded
   as `Classification::Noise`, not added to the diagram — this is what keeps
   whiteboard glare, texture, and stray marks from becoming spurious nodes.
   `detect_shapes(&GrayImage) -> Vec<ShapeCandidate>` runs the full
   binarize→contour→classify pipeline; `build_diagram(&GrayImage) -> Diagram`
   wraps that into diagram nodes (no labels, no edges yet). Tested with
   procedurally-generated fixtures, including scenes combining multiple
   shapes with scattered noise speckles and a near-full-frame border — not
   just clean single-shape images. **Known gap**: these are synthetic
   fixtures, not real photos; real whiteboard-photo fixtures in
   `tests/fixtures/` would be a valuable follow-up validation pass.
4. *(not yet implemented)* Line/arrow detection and edge-to-shape
   association: distinguish thin/open/elongated contours (lines) from the
   closed shapes above, detect an arrowhead (small triangular cap) for
   direction, and match endpoints to the nearest shape bounding boxes to
   build a graph edge.
5. *(not yet implemented)* OCR: crop each shape's interior and run `ocrs`
   (pure-Rust OCR via the `rten` ONNX runtime — confirmed to build for
   `wasm32-unknown-unknown`) to get its label.

### `d2gen` — Diagram → D2 text

Pure function `generate(&Diagram) -> String` (`crates/d2gen/src/lib.rs`).
Depends on `vision` only for the `Diagram`/`Node`/`Edge`/`ShapeKind` types —
no image/CV concerns. Shapes map to D2's `shape:` keyword; edges map to
`node0 -> node1: label`.

### `web` — Leptos CSR frontend

Not yet wired to `vision`/`d2gen`. Currently (`crates/web/src/main.rs`,
built with Trunk via `crates/web/index.html`): a "Start camera" button calls
`getUserMedia` and streams the result into a `<video>` element — real camera
capture, but nothing reads the frames yet.

Still to build: a hidden `<canvas>` to grab frames from the video, and **two
processing speeds** running concurrently — lightweight contour detection at
~5fps for a live bounding-box overlay (so the user gets instant feedback),
while the expensive OCR + full D2 regeneration only runs every ~1s or on an
explicit "capture" action. Running OCR at video framerate is not viable —
this two-speed split is a deliberate perf decision, not an implementation
shortcut to skip later.

D2→image rendering will call into `@terrastruct/d2` ("d2.js"), Terrastruct's
official WASM build of the D2 compiler, via a thin JS interop shim (planned:
`crates/web/js/d2-interop.js`) — `wasm-bindgen` can't call an npm package
directly, so a small JS glue layer is required even though everything still
runs client-side. This avoids needing a Go binary or server round-trip to
render diagrams.

### `server` — Axum static file server

Deliberately thin: serves the Trunk-built static bundle
(`tower_http::services::ServeDir`) plus a `/health` check. It exists only
because the product is framed as "a web app," not because any pipeline logic
runs server-side — all recognition stays client-side per the design above.

## Milestones

- **M0 — scaffold** (done): workspace, four crates compiling, CI, Leptos
  hello-world served by Axum.
- **M1 — offline vision pipeline** (in progress): contour extraction and
  shape classification (rectangle/circle/diamond, with noise filtering) are
  implemented and tested against procedural fixtures, including noisy scenes
  — see `vision`'s `detect_shapes`/`build_diagram` above. Still missing:
  arrow/connector detection, real-photo fixtures (only synthetic images
  tested so far), and golden D2 output tests. Deliberately not yet wired
  into `web`/the GUI — the user chose to get classification solid first
  rather than wire a naive/noisy pipeline into the UI early.
- **M2 — OCR**: integrate `ocrs` into the same offline pipeline/fixtures.
- **M3 — live camera wiring** (in progress): camera capture is done (`web`
  requests `getUserMedia` and shows the live feed); canvas frame grab, the
  two-speed processing loop, live shape overlay, and live-updating D2 text
  panel are not yet built — reusing `vision`/`d2gen` as-is once they are.
- **M4 — D2 rendering + export**: JS interop to `@terrastruct/d2` for a live
  SVG preview; "Export D2 source" / "Export image" actions.
- **M5 — polish & deploy**: handle empty/low-confidence detections,
  responsive layout, pick a deploy target, finish docs.
