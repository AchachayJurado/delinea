# delinea

Point a camera at a hand-drawn diagram — boxes, circles, diamonds, arrows,
labels — and delinea recognizes the shapes and text, and turns them into
[D2](https://d2lang.com) diagram-as-code, live, entirely in the browser.

## Status

Early scaffold. Shape/OCR recognition and the live camera pipeline are not
implemented yet — see `CLAUDE.md` for the architecture and milestone plan.

## Development

```sh
# run the pure-Rust pipeline tests (vision + d2gen), no browser/wasm needed
cargo test --workspace

# build the WASM frontend
cd crates/web && trunk build

# serve it (run from the repo root, after a trunk build)
cargo run -p server
# -> http://localhost:8080
```

See `CLAUDE.md` for full architecture, commands, and the phased build plan.
