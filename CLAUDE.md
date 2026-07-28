# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A Rust web app: point a camera at a hand-drawn diagram (boxes, circles,
diamonds, arrows, labels — on paper or a whiteboard) and it live-detects the
shapes/text and turns them into [D2](https://d2lang.com) diagram-as-code,
plus a rendered preview image. The entire pipeline — camera capture, CV
shape detection, OCR, D2 codegen, and D2→SVG rendering — runs client-side in
WASM; no frame is ever uploaded to a server.

**Current status**: early scaffold (M0 done, M1 started). The `vision` and
`d2gen` crates exist with a real (if minimal) first pipeline stage and are
unit-tested; the live camera loop, full shape classification, OCR, and D2
rendering are not implemented yet. See "Milestones" below for what's next.

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
cargo test -p vision finds_a_single_rectangle_contour
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
2. Contour extraction (`imageproc::contours::find_contours`) — **implemented**
   in `crates/vision/src/pipeline.rs::find_shape_regions`, returns bounding
   boxes of candidate regions. This is the only pipeline stage that exists so
   far.
3. *(not yet implemented)* Shape classification from the contour polygon:
   vertex count + angle + aspect ratio + fill ratio distinguishes
   rectangle/diamond/circle (closed, filled-ish) from line/arrow (thin, open,
   elongated). Arrowhead = small triangular cap at one end, used for edge
   direction.
4. *(not yet implemented)* Edge-to-shape association: match each line's
   endpoints to the nearest shape bounding boxes to build a graph edge.
5. *(not yet implemented)* OCR: crop each shape's interior and run `ocrs`
   (pure-Rust OCR via the `rten` ONNX runtime — confirmed to build for
   `wasm32-unknown-unknown`) to get its label.

### `d2gen` — Diagram → D2 text

Pure function `generate(&Diagram) -> String` (`crates/d2gen/src/lib.rs`).
Depends on `vision` only for the `Diagram`/`Node`/`Edge`/`ShapeKind` types —
no image/CV concerns. Shapes map to D2's `shape:` keyword; edges map to
`node0 -> node1: label`.

### `web` — Leptos CSR frontend

Not yet wired to `vision`/`d2gen` — currently a hello-world Leptos app
(`crates/web/src/main.rs`) built with Trunk (`crates/web/index.html`).

Planned design (see the plan in `.claude/plans` history, or M3/M4 below) for
when the camera loop lands: `getUserMedia` → `<video>`, a hidden `<canvas>`
grabs frames, and **two processing speeds** run concurrently — lightweight
contour detection at ~5fps for a live bounding-box overlay (so the user gets
instant feedback), while the expensive OCR + full D2 regeneration only runs
every ~1s or on an explicit "capture" action. Running OCR at video framerate
is not viable — this two-speed split is a deliberate perf decision, not an
implementation shortcut to skip later.

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
- **M1 — offline vision pipeline** (in progress): shape+contour detection
  against fixture images (no camera yet), verified with plain `cargo test`.
  Contour extraction is implemented; shape classification, arrow detection,
  and fixture images with golden D2 output are not yet added. This is the
  highest-risk part algorithmically — get it right before touching
  WASM/camera plumbing.
- **M2 — OCR**: integrate `ocrs` into the same offline pipeline/fixtures.
- **M3 — live camera wiring**: camera capture, canvas frame grab, the
  two-speed processing loop, live shape overlay, live-updating D2 text panel
  — reusing `vision`/`d2gen` as-is.
- **M4 — D2 rendering + export**: JS interop to `@terrastruct/d2` for a live
  SVG preview; "Export D2 source" / "Export image" actions.
- **M5 — polish & deploy**: handle empty/low-confidence detections,
  responsive layout, pick a deploy target, finish docs.
