# Rust Migration Plan: ALNview

## Executive Summary

**Goal**: Migrate ALNview from C/C++/Qt to Rust/egui while maintaining functionality and fixing buffer overflows.

**Strategy**: Ship of Theseus - replace components incrementally, shipping working code at each step.

**Timeline**: 7-14 weeks (depending on scope)

**Key Decision**: Replace Qt GUI first (Week 1), then gradually port C modules as needed.

---

## Why Migrate?

### Problems with Current Codebase
- ✗ Buffer overflows in C code (reported by maintainer)
- ✗ Qt dependency complexity (build system, cross-platform issues)
- ✗ Manual memory management risks
- ✗ Difficult to debug memory issues
- ✗ C error handling via global buffers (`Ebuffer`)

### Benefits of Rust Migration
- ✓ Memory safety (no buffer overflows, use-after-free, etc.)
- ✓ Modern error handling (`Result<T, E>`)
- ✓ Better tooling (`cargo`, `clippy`, `rustfmt`)
- ✓ Easier cross-compilation
- ✓ Thread safety (fearless concurrency)
- ✓ Simpler build system
- ✓ No Qt dependency

---

## Migration Strategy: Top-Down

**Key Insight**: Qt + Rust is painful. egui + C FFI is trivial.

Therefore: **Replace GUI first, then gradually port C modules.**

```
Phase 1: Rust GUI ──→ C backend (FFI)
Phase 2: Rust GUI ──→ Rust modules + C remainder
Phase 3: Rust GUI ──→ Pure Rust
```

This allows us to:
1. **Ship working code immediately** (after Week 1)
2. **Remove Qt dependency early** (no mixed Qt/Rust hell)
3. **Port C modules at leisure** (incrementally, as bugs are found)
4. **Test continuously** (every phase produces working binary)

---

## Phase 1: GUI Replacement (Week 1) - CRITICAL

### Objective
Replace Qt GUI with egui, calling existing C code via FFI. **Ship immediately.**

### Tasks

#### 1.1 Project Setup
- [ ] Create `Cargo.toml` with dependencies:
  - `eframe` (egui framework)
  - `egui` (immediate mode GUI)
  - `rfd` (native file dialogs)
- [ ] Create `build.rs` to compile C code
- [ ] Test that C code compiles from Rust build system

#### 1.2 Minimal FFI Bindings
- [ ] Write `src/ffi.rs` with manual bindings:
  - `DotPlot`, `DotLayer` (opaque types)
  - `createPlot()`, `Plot_Layer()`, `Free_List()`, `Free_DotPlot()`
  - `Frame`, `View`, `DotSegment`, `QuadLeaf` structs
- [ ] Test FFI: call `createPlot()` from Rust, verify it works

#### 1.3 Basic GUI Implementation
- [ ] `src/main.rs`: eframe app skeleton
- [ ] File open dialog (using `rfd`)
- [ ] Call `createPlot()` when file selected
- [ ] Basic canvas rendering (allocate painter)
- [ ] Coordinate transformation (genomic → screen)

#### 1.4 Core Rendering
- [ ] Call `Plot_Layer()` for current view frame
- [ ] Iterate segments, draw lines with egui painter
- [ ] Implement zoom (mouse scroll or buttons)
- [ ] Implement pan (drag)

#### 1.5 UI Controls
- [ ] Top menu bar (Open, Zoom +/-, Reset)
- [ ] Side panel for layer controls:
  - Visibility checkboxes
  - Color pickers (forward/reverse)
  - Thickness sliders
- [ ] Coordinate display labels

#### 1.6 Testing & Polish
- [ ] Test with real `.1aln` files
- [ ] Verify performance (should match Qt version)
- [ ] Fix coordinate transform bugs
- [ ] Handle window resize correctly

### Deliverable
**Working Rust+egui GUI with no Qt dependency, calling C backend.**

### Files Created
```
Cargo.toml
build.rs
src/
├── main.rs        (~300 lines - main app)
├── ffi.rs         (~100 lines - C bindings)
└── gui.rs         (optional - split UI code)
```

### Success Criteria
- [ ] `cargo build` succeeds
- [ ] Opens `.1aln` files
- [ ] Displays alignment segments correctly
- [ ] Zoom/pan works
- [ ] No crashes
- [ ] No Qt libraries required

---

## Phase 2: Bug Hunting & Stabilization (Week 2)

### Objective
Find and document all buffer overflows and memory issues in C code.

### Tasks

#### 2.1 Enable Sanitizers
- [ ] Update `build.rs` with conditional ASAN build:
  ```rust
  if cfg!(debug_assertions) {
      cc.flag("-fsanitize=address");
      cc.flag("-fsanitize=undefined");
      cc.flag("-g");
  }
  ```
- [ ] Test build: `cargo build`
- [ ] Run with ASAN: `cargo run`

#### 2.2 Systematic Testing
- [ ] Test file loading with various `.1aln` files
- [ ] Test with corrupted/invalid files (trigger error paths)
- [ ] Test zoom to extreme levels
- [ ] Test with very large files
- [ ] Monitor ASAN output for:
  - Buffer overflows
  - Use-after-free
  - Memory leaks
  - Undefined behavior

#### 2.3 Document Issues
- [ ] Create `BUGS.md` with all findings
- [ ] Prioritize by severity:
  - P0: Crashes/data corruption
  - P1: Memory leaks
  - P2: Performance issues
- [ ] Note which modules need porting first

#### 2.4 Valgrind Analysis (if needed)
```bash
cargo build
valgrind --leak-check=full --track-origins=yes \
  target/debug/alnview test.1aln
```

### Deliverable
**Comprehensive bug report, priority list for module porting.**

---

## Phase 3: Port GDB Module (Weeks 3-4)

### Objective
Replace `GDB.c` with safe Rust implementation. **High priority - handles sequence data.**

### Why GDB First?
- Self-contained module
- Known buffer overflow risks (sequence handling)
- Clear API boundary
- Critical for data integrity

### Tasks

#### 3.1 Design Rust API
- [ ] Define `src/gdb.rs` module
- [ ] Design safe structs:
  ```rust
  pub struct GenomeDB { ... }
  pub struct Scaffold { ... }
  pub struct Contig { ... }
  ```
- [ ] Use `Vec<u8>` for sequences (bounds-checked)
- [ ] Use `Result<T, GdbError>` for error handling

#### 3.2 Implement Core Functions
- [ ] `GenomeDB::load()` - parse `.1seq` format
- [ ] `get_scaffold()` / `get_contig()` - safe accessors
- [ ] `get_sequence()` - bounds-checked sequence retrieval
- [ ] Handle 2-bit compression safely

#### 3.3 Parser Implementation
Use `nom` parser combinator or manual parsing:
- [ ] Parse ONE file headers
- [ ] Parse scaffold records
- [ ] Parse contig records
- [ ] Parse sequence data (DNA compression)
- [ ] Validate all offsets before use

#### 3.4 FFI Bridge (Temporary)
Create C-compatible wrapper to maintain compatibility:
```rust
#[no_mangle]
pub extern "C" fn Read_GDB(
    gdb_out: *mut ffi::GDB,
    path: *const c_char
) -> i32 {
    // Call Rust implementation
    // Convert back to C struct for FFI
}
```

#### 3.5 Testing
- [ ] Unit tests for all parsing functions
- [ ] Test with known-good `.1seq` files
- [ ] Test error conditions (corrupted files)
- [ ] Fuzz test with random data
- [ ] Verify no ASAN errors
- [ ] Benchmark vs C version (should be similar)

#### 3.6 Integration
- [ ] Update FFI layer to use Rust GDB
- [ ] Remove `GDB.c` from `build.rs`
- [ ] Test full application
- [ ] Verify no regressions

### Deliverable
**Safe GDB module in Rust, no buffer overflows, C code removed.**

---

## Phase 4: Port DotPlot & Quad-Tree (Weeks 5-6)

### Objective
Replace `sticks.c` quad-tree and segment management with safe Rust.

### Tasks

#### 4.1 Data Structures
- [ ] `src/dotplot.rs` module
- [ ] `DotPlot` struct
- [ ] `Layer` struct
- [ ] `Segment` struct (simple, Copy-able)

#### 4.2 Quad-Tree Implementation
```rust
pub struct QuadTree {
    root: Box<QuadNode>,
}

enum QuadNode {
    Leaf { segments: Vec<usize> },
    Branch { bounds: Rect, children: [Box<QuadNode>; 4] },
}
```

- [ ] Implement recursive build algorithm
- [ ] Implement query algorithm
- [ ] Add tests for boundary cases
- [ ] Benchmark query performance

#### 4.3 Segment Loading
- [ ] Parse `.1aln` format in Rust
- [ ] Build segment array
- [ ] Build quad-tree from segments
- [ ] Apply filtering (length, identity, size cutoffs)

#### 4.4 Integration
- [ ] Update FFI to expose Rust DotPlot
- [ ] Update rendering to use Rust queries
- [ ] Remove `sticks.c` from build
- [ ] Verify performance (should be faster!)

### Deliverable
**Safe quad-tree in Rust, faster queries, no C pointer issues.**

---

## Phase 5: ONElib Strategy Decision (Week 7)

### Objective
Decide how to handle ONElib dependency (130k LOC).

### Option A: Keep as FFI (Recommended Initially)
**Pros:**
- Already works
- Minimal effort
- Focus on other modules

**Cons:**
- Still unsafe C code
- Harder to debug issues

**Implementation:**
- [ ] Create safe Rust wrapper around C API
- [ ] Use RAII for resource cleanup:
  ```rust
  pub struct OneFile {
      handle: *mut ffi::OneFile,
  }
  impl Drop for OneFile {
      fn drop(&mut self) {
          unsafe { ffi::oneFileClose(self.handle) }
      }
  }
  ```

### Option B: Port to Rust
**Pros:**
- Full safety
- Better error handling
- Can optimize for ALNview use case

**Cons:**
- 2-3 months work
- Need to understand compression codecs
- Risk of incompatibility

**Implementation (if chosen):**
- [ ] Study ONE format specification
- [ ] Implement schema parser
- [ ] Implement line-by-line reader
- [ ] Implement compression codecs
- [ ] Extensive compatibility testing

### Option C: Replace with Rust Serialization
**Pros:**
- Use proven libraries (`serde`, `bincode`)
- Simpler code

**Cons:**
- Lose compatibility with existing `.1aln` files
- Not acceptable for this project

### Decision Point
**Recommendation: Option A initially, Option B later if needed.**

---

## Phase 6: Alignment Module (Weeks 8-9)

### Objective
Port `align.c` to Rust for safe alignment computation.

### Tasks

#### 6.1 Core Structures
- [ ] Port `Path`, `Alignment` structs
- [ ] Use Rust slices instead of raw pointers
- [ ] Safe trace buffer management

#### 6.2 Algorithm Implementation
- [ ] Port `Compute_Trace_PTS()`
- [ ] Port `Compute_Trace_MID()`
- [ ] Port alignment display formatting
- [ ] Ensure no buffer overflows in trace computation

#### 6.3 Work Data Management
Replace manual memory pools with Rust allocations:
- [ ] Use `Vec` for dynamic buffers
- [ ] Thread-local storage if needed
- [ ] Let Rust handle cleanup

#### 6.4 Testing
- [ ] Unit tests for trace computation
- [ ] Compare output to C version
- [ ] Test edge cases (very long alignments, gaps, etc.)

### Deliverable
**Safe alignment computation, no trace buffer overflows.**

---

## Phase 7: Remaining Modules (Weeks 10-12)

### Modules to Port (in order)

#### 7.1 Hash Table (`hash.c`)
- Simple, port quickly
- Use `std::collections::HashMap` or custom if needed
- [ ] Implement
- [ ] Test
- [ ] Remove C version

#### 7.2 K-mer Dot Plot (`doter.c`)
- [ ] Port k-mer extraction
- [ ] Safe hash table
- [ ] Bounds checking on raster writes
- [ ] Test performance

#### 7.3 Selection Parser (`select.c`)
- [ ] Port grammar parser (use `nom`)
- [ ] Safe string handling
- [ ] Test with various selection expressions

#### 7.4 Alignment Encoding (`alncode.c`)
- [ ] Port compression/decompression
- [ ] Safe bit manipulation
- [ ] Validate all bitwise operations

#### 7.5 Utility Functions (`gene_core.c`)
- [ ] Port as needed (many may not be used)
- [ ] Use Rust standard library where possible
- [ ] Safe string formatting

### Deliverable
**All modules ported to Rust, C code completely removed.**

---

## Phase 8: Polish & Optimization (Weeks 13-14)

### Tasks

#### 8.1 Performance Optimization
- [ ] Profile with `cargo flamegraph`
- [ ] Optimize hot paths
- [ ] Consider SIMD for sequence operations
- [ ] Parallel quad-tree queries if beneficial

#### 8.2 Error Handling Cleanup
- [ ] Consistent error types across modules
- [ ] User-friendly error messages in GUI
- [ ] Logging framework (use `tracing` crate)

#### 8.3 Testing
- [ ] Integration tests
- [ ] Fuzz testing critical parsers
- [ ] Test on all platforms (Linux, Mac, Windows)
- [ ] Memory leak testing (should be none!)

#### 8.4 Documentation
- [ ] API documentation (`cargo doc`)
- [ ] Update README with Rust build instructions
- [ ] Migration notes for users

#### 8.5 Packaging
- [ ] Create release builds
- [ ] GitHub Actions for CI/CD
- [ ] Binary releases for platforms
- [ ] Consider `cargo-dist` for distribution

### Deliverable
**Production-ready Rust application, ready to replace C/Qt version.**

---

## Risk Mitigation

### Technical Risks

| Risk | Mitigation |
|------|------------|
| Performance regression | Benchmark each phase, optimize hot paths |
| FFI bugs | Extensive testing, ASAN during development |
| ONElib incompatibility | Keep C version as fallback initially |
| Missing Qt features | Verify feature parity before starting |
| Rust learning curve | Start simple, reference Rust book, ask community |

### Project Risks

| Risk | Mitigation |
|------|------------|
| Timeline overrun | Ship incrementally, Week 1 already valuable |
| Scope creep | Focus on critical path, defer nice-to-haves |
| Abandonment | Each phase produces working code |
| User resistance | Maintain compatibility, smooth transition |

---

## Success Metrics

### Phase 1 (GUI)
- [ ] Builds on Linux, Mac, Windows
- [ ] Opens `.1aln` files
- [ ] Renders alignments correctly
- [ ] Zoom/pan works smoothly
- [ ] No crashes in 1 hour of testing

### Phase 3 (GDB)
- [ ] No ASAN errors with any `.1seq` file
- [ ] Passes all C version test cases
- [ ] Performance within 10% of C version

### Phase 4 (Quad-Tree)
- [ ] Query performance same or better than C
- [ ] Handles 1M+ segments without issues
- [ ] No memory leaks

### Final (Complete)
- [ ] Zero unsafe code in main application
- [ ] All tests pass
- [ ] No memory leaks (valgrind clean)
- [ ] Startup time < 2 seconds
- [ ] Binary size < 50MB
- [ ] Works on Linux, Mac, Windows

---

## Development Environment

### Required Tools
```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Development tools
cargo install cargo-watch    # Auto-rebuild
cargo install cargo-flamegraph  # Profiling
cargo install cargo-fuzz     # Fuzzing

# Sanitizer support
rustup component add rust-src
```

### Recommended VS Code Extensions
- `rust-analyzer` (LSP)
- `CodeLLDB` (debugging)
- `crates` (dependency management)

### Build Configurations

**Development:**
```bash
cargo build  # Debug mode, ASAN enabled
cargo run
```

**Release:**
```bash
cargo build --release
strip target/release/alnview  # Reduce size
```

**Testing:**
```bash
cargo test
cargo test --release  # Faster tests
```

---

## File Organization (Final State)

```
alnview/
├── Cargo.toml
├── build.rs                # Only if keeping C code temporarily
├── src/
│   ├── main.rs            # Application entry, egui setup
│   ├── gui/
│   │   ├── mod.rs         # GUI module
│   │   ├── canvas.rs      # Main rendering canvas
│   │   ├── controls.rs    # Layer controls, menus
│   │   └── dialogs.rs     # File open, etc.
│   ├── core/
│   │   ├── mod.rs
│   │   ├── gdb.rs         # Genome database
│   │   ├── dotplot.rs     # Dot plot model
│   │   ├── quadtree.rs    # Spatial index
│   │   └── segment.rs     # Alignment segments
│   ├── io/
│   │   ├── mod.rs
│   │   ├── one.rs         # ONE format (or FFI wrapper)
│   │   └── parsers.rs     # Selection expressions, etc.
│   ├── align/
│   │   ├── mod.rs
│   │   ├── trace.rs       # Trace computation
│   │   └── display.rs     # Alignment formatting
│   └── ffi.rs             # C bindings (if any remain)
├── tests/
│   ├── integration.rs
│   └── data/              # Test .1aln files
├── benches/
│   └── quadtree.rs        # Performance benchmarks
├── DESIGN.md              # Architecture documentation
├── PLAN.md                # This file
└── README.md              # User documentation
```

---

## Alternative: Minimal Viable Migration

**If timeline is tight, consider:**

### Week 1-2: GUI + ASAN
- Replace Qt with egui
- Enable ASAN on C code
- Ship with safer error detection

### Week 3+: Port only broken modules
- Use ASAN to find crashes
- Port only the modules with bugs
- Leave working C code alone

**This gets you:**
- No Qt dependency
- Memory safety monitoring
- Fix actual bugs found
- Ship faster

---

## Decision Points

### Week 1 Decision
**After GUI replacement:** Is egui acceptable?
- **Yes** → Continue with plan
- **No** → Evaluate alternative (iced, Slint)

### Week 7 Decision
**ONElib strategy:** FFI or port?
- **If C version stable** → Keep FFI
- **If bugs found** → Port to Rust
- **If performance critical** → Port to Rust

### Week 10 Decision
**Remaining C code:** Port all or keep some?
- **If stable utility code** → Keep via FFI
- **If critical path** → Port to Rust
- **If niche features** → Evaluate usage

---

## Resources

### Rust Learning
- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [egui Documentation](https://docs.rs/egui/)

### FFI Resources
- [Rust FFI Omnibus](http://jakegoulding.com/rust-ffi-omnibus/)
- [The Rustonomicon](https://doc.rust-lang.org/nomicon/)

### Parser Libraries
- [`nom`](https://docs.rs/nom/) - Parser combinators
- [`pest`](https://docs.rs/pest/) - PEG parser

### Serialization
- [`serde`](https://serde.rs/) - Serialization framework
- [`bincode`](https://docs.rs/bincode/) - Binary encoding

---

## Questions to Answer

Before starting:
- [ ] Is cross-platform support required? (affects GUI choice)
- [ ] What's the acceptable timeline?
- [ ] Can we ship Rust version alongside C version initially?
- [ ] Are there automated tests we can preserve?
- [ ] What's the minimum feature set for v1.0?

---

## Rollback Plan

**If migration fails:**
- Keep original C/Qt code in `legacy/` branch
- FFI approach allows gradual rollback
- Can ship Rust GUI + C backend indefinitely
- No "all or nothing" commitment

---

## Summary

**Start:** Week 1 - Replace Qt GUI with egui
**Milestone 1:** Working Rust GUI (Week 1)
**Milestone 2:** Critical modules safe (Week 6)
**Milestone 3:** Fully Rust application (Week 12)
**Complete:** Polished, optimized, shipped (Week 14)

**Key Principle:** Ship working code every week. No big-bang rewrite.

**Next Steps:**
1. Review this plan
2. Set up Rust development environment
3. Create starter files (Week 1 tasks)
4. Begin GUI migration

---

**Let's build something safer. 🦀**
