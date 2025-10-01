# ALNview - Rust Edition 🦀

This is the Rust rewrite of ALNview, replacing Qt with egui while maintaining the C backend via FFI.

## Quick Start

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Linux: Install dependencies
sudo apt-get install libgtk-3-dev  # For file dialogs

# macOS: No extra deps needed

# Windows: Install Visual Studio Build Tools
```

### Build & Run

```bash
# Normal build
cargo build
cargo run

# Release build (optimized)
cargo build --release
./target/release/alnview

# With ASAN (address sanitizer for bug hunting)
ASAN=1 cargo build
ASAN_OPTIONS=detect_leaks=1 cargo run
```

## Development

### Project Structure

```
alnview/
├── Cargo.toml       # Rust dependencies
├── build.rs         # Compiles C code
├── src/
│   ├── main.rs      # egui GUI + app logic
│   └── ffi.rs       # C FFI bindings
├── *.c / *.h        # Existing C backend (temporary)
└── target/          # Build output
```

### Adding Dependencies

Edit `Cargo.toml`:

```toml
[dependencies]
your-crate = "1.0"
```

Then `cargo build` will fetch it automatically.

### Hot Reload

```bash
# Install cargo-watch
cargo install cargo-watch

# Auto-rebuild on file changes
cargo watch -x run
```

## Current Status

✅ **Working:**
- egui GUI framework
- File dialog (native)
- C code compilation via FFI
- Basic plot loading structure
- Zoom/pan/reset
- Layer controls UI

🚧 **In Progress:**
- Actual segment rendering (need to properly access C segment data)
- Getting genome lengths from plot
- Multiple layers support
- K-mer dot plot

📋 **TODO:**
- See PLAN.md for full migration roadmap

## Debugging

### ASAN (Address Sanitizer)

Find buffer overflows in C code:

```bash
ASAN=1 cargo build
ASAN_OPTIONS=detect_leaks=1 cargo run yourfile.1aln
```

### Logging

```bash
RUST_LOG=debug cargo run
```

### GDB/LLDB

```bash
rust-gdb ./target/debug/alnview
# or
rust-lldb ./target/debug/alnview
```

## Differences from Qt Version

| Feature | Qt Version | Rust Version |
|---------|-----------|--------------|
| GUI Framework | Qt 6.9+ | egui |
| Build System | qmake | cargo |
| File Dialogs | Qt | rfd (native) |
| Memory Safety | Manual | Rust (safe by default) |
| Error Handling | Global buffer | Result<T, E> |
| Cross-platform | Qt libs | Rust stdlib |

## Performance

Should be similar to Qt version:
- C backend unchanged (same speed)
- egui rendering is very fast (retained mode under the hood)
- Rust adds zero overhead

## Next Steps

See **PLAN.md** for the full migration plan.

**Phase 1 (current):** Get basic rendering working
**Phase 2:** Port GDB module to Rust (fix buffer overflows)
**Phase 3:** Port quad-tree to Rust
**Phase 4+:** Gradually replace remaining C modules

## License

Same as original ALNview.

## Questions?

Check the docs:
- `DESIGN.md` - Architecture deep dive
- `PLAN.md` - Migration roadmap
- [egui docs](https://docs.rs/egui/)
- [The Rust Book](https://doc.rust-lang.org/book/)
