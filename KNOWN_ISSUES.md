# Known Issues - Rust Migration

## Critical: File Loading Hangs UI (Week 1)

**Status**: 🐛 In Progress  
**Priority**: P0  
**Affects**: All file loading

### Problem

When trying to load a `.1aln` file, the application appears to freeze/lock up.

### Root Cause

The C function `createPlot()` is synchronous and does heavy I/O:
1. Opens `.1aln` file (ONE format parsing)
2. Loads TWO genome databases (`.1seq` files - can be GBs)
3. Reads all alignment records
4. Builds quad-tree spatial indices

This can take **seconds to minutes** for large files, blocking the UI thread.

### Current Workaround

Added async loading with spinner, but **C pointers aren't thread-safe** (`!Send`), so we can't actually transfer the loaded plot back to the main thread yet.

### Attempted Solutions

1. ✅ Added `LoadingState` enum to track progress
2. ✅ Added spinner UI to show loading
3. ✅ Spawned background thread
4. ❌ Can't transfer `*mut DotPlot` between threads (not `Send`)

### Proper Solutions (in order of preference)

#### Option A: Implement in Rust (Best)
Port the loading logic to safe Rust:
- Parse ONE format natively (Week 5-7 in PLAN.md)
- Make thread-safe by design
- Show real progress bar
- Interruptible loading

**Timeline**: Weeks 5-7  
**Complexity**: Medium  
**Benefits**: Full safety, progress tracking, cancellation

#### Option B: Use Channels
Keep C code, but use channels to communicate:
```rust
use std::sync::mpsc;

let (tx, rx) = mpsc::channel();
thread::spawn(move || {
    let plot = createPlot(...);
    tx.send(plot).unwrap();
});

// In update():
if let Ok(plot) = rx.try_recv() {
    self.plot = Some(SafePlot::new(plot));
}
```

**Timeline**: 1 day  
**Complexity**: Low  
**Benefits**: Quick fix  
**Drawbacks**: Still unsafe, no progress tracking

#### Option C: Make C Code Callback-based
Modify C code to call Rust callbacks during loading:
```c
void createPlot(..., void (*progress)(int percent)) {
    // ...
    progress(25);
    // ...
}
```

**Timeline**: 2-3 days  
**Complexity**: Medium  
**Benefits**: Progress tracking  
**Drawbacks**: Modifying C code (defeats purpose)

### Recommendation

**Short term (this week)**: Implement Option B (channels)  
**Long term (Weeks 5-7)**: Implement Option A (Rust parser)

### Testing

To reproduce:
```bash
cargo run
# Click "Open File"
# Select any .1aln file
# App freezes/shows spinner indefinitely
```

Watch terminal output - you'll see:
```
🔍 Starting async load: ...
🧵 Background thread: Loading file...
📞 Calling C createPlot()...
📞 C createPlot() returned: 0x...
⚠️  WARNING: Plot loaded in background thread but we can't transfer it to UI thread yet!
```

### References

- PLAN.md - Phase 5 (ONElib strategy)
- DESIGN.md - DotPlot loading section
- src/main.rs:455 - `load_file_async()` function

---

**Last Updated**: 2025-10-01  
**Assignee**: Week 2 work  
**Blocker For**: Actual usage of the app
