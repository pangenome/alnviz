# ALNview Architecture and Design

## Table of Contents
1. [System Overview](#system-overview)
2. [Architecture Overview](#architecture-overview)
3. [Data Flow](#data-flow)
4. [Core Components](#core-components)
5. [Data Structures](#data-structures)
6. [File Format Support](#file-format-support)
7. [Visualization System](#visualization-system)
8. [Interaction Model](#interaction-model)
9. [Memory Management](#memory-management)
10. [Build System](#build-system)

---

## System Overview

ALNview is a Qt-based graphical alignment viewer designed for visualizing genomic alignments stored in the `.1aln` format (part of the ONE file format specification). The application enables interactive exploration of alignment data between two genomic sequences through a dot-plot visualization paradigm.

### Key Features
- **Multi-layer visualization**: Display multiple `.1aln` files as overlay layers
- **Interactive zooming**: Pan and zoom through genomic coordinates
- **K-mer dot plots**: Generate true k-mer dot plots when zoomed below 1Mbp resolution
- **Alignment picking**: Select individual alignment segments to view detailed alignments
- **Customizable rendering**: Control line thickness, colors, and visibility per layer
- **Coordinate systems**: Multiple coordinate display formats (nucleotide, contig, scaffold, etc.)

---

## Architecture Overview

ALNview follows a **layered architecture** combining Qt's MVC (Model-View-Controller) pattern with a traditional C data processing backend:

```
┌─────────────────────────────────────────────────────────────┐
│                    Qt GUI Layer (C++)                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ DotWindow    │  │ OpenDialog   │  │ AlignWindow  │     │
│  │ (QMainWindow)│  │ (QDialog)    │  │ (QMainWindow)│     │
│  └──────┬───────┘  └──────┬───────┘  └──────────────┘     │
│         │                  │                                │
│  ┌──────▼──────────────────▼────────┐                      │
│  │     DotCanvas (QWidget)          │                      │
│  │  (Main visualization surface)    │                      │
│  └──────┬───────────────────────────┘                      │
└─────────┼────────────────────────────────────────────────┘
          │
┌─────────▼────────────────────────────────────────────────┐
│              State Management (C++)                       │
│  ┌──────────────┐                                         │
│  │  DotState    │  (View parameters, layers, zoom stack) │
│  └──────┬───────┘                                         │
└─────────┼───────────────────────────────────────────────┘
          │
┌─────────▼────────────────────────────────────────────────┐
│          Data Model & Processing (C)                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │  DotPlot     │  │  DotLayer    │  │  DotGDB      │   │
│  │  (model)     │  │  (alignments)│  │  (sequences) │   │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘   │
└─────────┼──────────────────┼──────────────────┼─────────┘
          │                  │                  │
┌─────────▼──────────────────▼──────────────────▼─────────┐
│              Core Libraries (C)                          │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐        │
│  │  ONElib    │  │    GDB     │  │   align    │        │
│  │ (file I/O) │  │ (genome DB)│  │ (alignment)│        │
│  └────────────┘  └────────────┘  └────────────┘        │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐        │
│  │   sticks   │  │   doter    │  │    hash    │        │
│  │ (plotting) │  │ (k-mer dot)│  │  (tables)  │        │
│  └────────────┘  └────────────┘  └────────────┘        │
└──────────────────────────────────────────────────────────┘
```

---

## Data Flow

### Application Startup
1. **main.cpp**: Initialize Qt application, create `OpenDialog`
2. **OpenDialog**: User selects `.1aln` file and filtering parameters
3. **sticks.c:createPlot()**: Load alignment file and create `DotPlot` structure
   - Parse ONE file format headers and provenance
   - Load genome databases (GDB) for both sequences
   - Read alignment records and build segment arrays
   - Construct quad-tree spatial index for efficient querying
4. **DotWindow**: Create main window with canvas and controls
5. **DotCanvas**: Initialize visualization surface

### Visualization Pipeline
```
User Interaction → View Update → Data Query → Rendering
```

#### 1. View Update
- User zooms/pans via mouse or keyboard
- **DotWindow** updates `DotState` (view coordinates, zoom level)
- **DotCanvas::viewToFrame()** converts view coordinates to frame coordinates

#### 2. Data Query
- **DotCanvas** calls **Plot_Layer()** for each visible layer
- **sticks.c:Plot_Layer()** queries quad-tree with current frame
- Returns `QuadLeaf` list containing relevant `DotSegment` records
- If zoomed < 1Mbp, **doter.c:dotplot()** generates k-mer dot plot

#### 3. Rendering
- **DotCanvas::paintEvent()** iterates through layers
- For each segment: draw line from (abeg, bbeg) to (aend, bend)
- Apply layer-specific color, thickness, and orientation
- Render k-mer dots if applicable
- Draw focus crosshairs and locator rectangle

### Alignment Display
1. User clicks on alignment segment
2. **DotCanvas::pick()** hit-tests segments near click point
3. **sticks.c:create_alignment()** loads trace data
4. **align.c:Compute_Trace_PTS()** computes exact alignment
5. **AlignWindow** displays formatted alignment text

---

## Core Components

### 1. **main.cpp** (Application Entry)
- Minimal Qt application bootstrap
- Creates global `OpenDialog` instance
- Calls `DotWindow::openFile()` to start workflow

### 2. **OpenDialog** (open_window.cpp/h)
**Purpose**: File selection and filtering dialog

**Key Features**:
- File browser for `.1aln` files
- Filtering options:
  - **Longest cutoff**: Minimum alignment length
  - **Identity cutoff**: Minimum percent identity
  - **Size cutoff**: Filter by alignment size

**State Management**:
- `Open_State` structure persists dialog settings
- Settings saved/restored via Qt's QSettings

### 3. **DotWindow** (main_window.cpp/h)
**Purpose**: Main application window (QMainWindow)

**Responsibilities**:
- Menu bar and toolbar management
- Layer control panel (enable/disable, colors, thickness)
- Coordinate display and input fields
- Zoom controls and focus management
- Window management (tile, cascade, raise all)

**Key Data**:
```cpp
DotState state;        // Current view state
DotPlot *plot;         // Data model
Frame *frame;          // Current visible frame
DotCanvas *canvas;     // Rendering widget
```

**Major Methods**:
- `openFile()`: Static method to launch file open dialog
- `zoomUp()/zoomDown()`: Adjust zoom level
- `viewChange()`: Update view from coordinate inputs
- `frameToView()`: Update coordinate displays from frame changes

### 4. **DotCanvas** (main_window.cpp/h)
**Purpose**: Custom Qt widget for rendering dot plot

**Responsibilities**:
- Paint alignment segments and k-mer dots
- Handle mouse events (pan, zoom, pick)
- Manage zoom stack (for zoom-out functionality)
- Display locator overview rectangle

**Rendering Strategy**:
- Uses QImage with indexed color table for fast pixel manipulation
- Draws directly into raster buffer for k-mer dots
- Uses QPainter for line segments (with anti-aliasing optional)

**Event Handling**:
- **Mouse press**: Start rubber-band zoom selection or segment picking
- **Mouse move**: Update cursor coordinates, show tooltip
- **Mouse release**: Execute zoom or display segment popup
- **Key press**: Arrow keys for panning, +/- for zoom, space for picking

### 5. **DotPlot** (sticks.h/c)
**Purpose**: Core data model for alignment visualization

**Structure**:
```c
typedef struct {
    int64 alen, blen;           // Sequence lengths
    DotGDB *db1, *db2;          // Genome databases
    int nlays;                   // Number of layers
    DotLayer *layers[MAX_LAYERS]; // Array of alignment layers
    void *dotmemory;            // Memory pool for k-mer dots
} DotPlot;
```

**Key Functions**:
- `createPlot()`: Load `.1aln` file, build data structures
- `copyPlot()`: Create independent copy (for multiple windows)
- `Plot_Layer()`: Query segments within frame bounds
- `Free_DotPlot()`: Cleanup memory

### 6. **DotLayer** (sticks.h/c)
**Purpose**: Single alignment layer (one `.1aln` file)

**Structure**:
```c
typedef struct {
    int nref;                   // Reference count
    char *name;                 // Layer name (file path)
    OneFile *input;             // ONE file handle
    int64 novls;                // Number of alignments
    int tspace;                 // Trace point spacing
    DotSegment *segs;           // Array of segments
    QuadNode *qtree;            // Spatial index
    QuadNode *blocks;           // Memory blocks for tree
} DotLayer;
```

**Quad-Tree Indexing**:
- Hierarchical spatial partitioning of alignment space
- Each node subdivides into 4 quadrants
- Leaves contain indices into segment array
- Enables O(log n) query for visible segments

### 7. **DotSegment** (sticks.h)
**Purpose**: Individual alignment segment record

```c
typedef struct {
    int64 abeg, aend;   // A-sequence interval
    int64 bbeg, bend;   // B-sequence interval
    int16 iid;          // Internal ID
    int16 mark;         // Flags (forward/reverse)
    int idx;            // Index in segment array
} DotSegment;
```

### 8. **GDB (Genome Database)** (GDB.h/c)
**Purpose**: Manage genomic sequence data

**Structure**:
```c
typedef struct {
    int nscaff;                 // Number of scaffolds
    GDB_SCAFFOLD *scaffolds;    // Scaffold records
    int ncontig;                // Number of contigs
    GDB_CONTIG *contigs;        // Contig records
    char *headers;              // Scaffold/contig names
    int64 seqtot;               // Total bases
    int seqstate;               // Format: compressed/numeric/ASCII
    void *seqs;                 // Sequence data (memory or file)
    float freq[4];              // Base frequencies (ACGT)
} GDB;
```

**Sequence Storage Modes**:
- **EXTERNAL**: Sequences on disk (`.bps` file)
- **COMPRESSED**: 2-bit encoding in memory (4 bases/byte)
- **NUMERIC**: Numeric array (ACGT = 0123)
- **LOWER_CASE/UPPER_CASE**: ASCII representation

**Key Functions**:
- `Read_GDB()`: Load GDB from `.1seq` file
- `Load_Sequences()`: Bring sequences into memory
- `Get_Contig()`: Retrieve contig sequence
- `Get_Contig_Piece()`: Retrieve sub-interval

### 9. **ONElib** (ONElib.h/c)
**Purpose**: Universal file I/O for ONE file format

**ONE File Format**:
- Schema-based binary/text format
- Self-describing with provenance tracking
- Compressed list fields with trained codecs
- Used for both `.1aln` (alignments) and `.1seq` (sequences)

**Key Types**:
- `OneFile`: File handle with schema and buffers
- `OneSchema`: Type definitions and metadata
- `OneCodec`: Compression codec instances

### 10. **align** (align.h/c)
**Purpose**: Local alignment computation and representation

**Key Abstractions**:
- **Path**: Alignment path with trace points
- **Alignment**: Path + sequence pointers + metadata
- **Overlap**: Serializable alignment record

**Trace Representation**:
- Sparse trace points at regular intervals (typically 100bp)
- Each point records: position and cumulative differences
- Enables efficient recomputation of exact alignment

**Key Functions**:
- `Local_Alignment()`: Find optimal local alignment
- `Compute_Trace_PTS()`: Compute exact trace from points
- `Print_Alignment()`: Format alignment for display
- `Complement_Seq()`: Reverse-complement sequence

### 11. **doter** (doter.h/c)
**Purpose**: K-mer dot plot generation

**Algorithm**:
1. Extract all k-mers from A-sequence in view
2. Hash k-mers into position table
3. Scan B-sequence, lookup each k-mer
4. Mark matching positions in raster
5. Return dot list for rendering

**Performance**:
- Only activated when view width < 1Mbp (both dimensions)
- Limited to MAX_DOTPLOT (1,000,000) dots to prevent memory overflow
- Uses simple hash table for k-mer lookup

### 12. **sticks** (sticks.h/c)
**Purpose**: Coordinate mapping and plot management

**Coordinate Formats**:
- **FORMAT_n**: Nucleotide position (e.g., "12345")
- **FORMAT_c**: Contig-relative (e.g., ".3:456")
- **FORMAT_s**: Scaffold-relative (e.g., "@2:1234")
- **FORMAT_s_c**: Scaffold + contig (e.g., "@2.3:456")
- **FORMAT_i**: Named scaffold (e.g., "@chr1:1234")
- **FORMAT_i_c**: Named scaffold + contig (e.g., "@chr1.3:456")

**Smart Formatting**:
- Automatically selects units (bp, kb, Mb, Gb)
- Adjusts precision based on view scale

### 13. **hash** (hash.h/c)
**Purpose**: Generic string hash table

**Features**:
- Dynamic resizing (progressive doubling)
- Optional string ownership (copy vs. reference)
- Used for scaffold name lookup

### 14. **select** (select.h/c)
**Purpose**: Genomic region selection parser

**Selection Syntax**:
- Range expressions: "scaffold1:1000-2000"
- Contig selection: ".3" (third contig)
- Scaffold selection: "@2" (second scaffold)
- Named scaffolds: "@chrX:1000-2000"

**Use Case**: Enables future extensions for sub-region viewing

---

## Data Structures

### View State Management

```c
typedef struct {
    QRect wGeom;                    // Window geometry

    View view;                      // Current view coordinates
    double zoom;                    // Current zoom level
    int format;                     // Coordinate display format

    bool fOn;                       // Focus enabled
    Focus focus;                    // Focus point (x, y)
    QColor fColor;                  // Focus color
    bool fViz;                      // Focus crosshairs visible

    int nlays;                      // Number of layers
    int order[MAX_LAYERS];          // Layer draw order
    bool on[MAX_LAYERS];            // Layer visibility
    QColor colorF[MAX_LAYERS];      // Forward strand colors
    QColor colorR[MAX_LAYERS];      // Reverse strand colors
    int thick[MAX_LAYERS];          // Line thickness

    QColor lColor;                  // Locator color
    bool lViz;                      // Locator visible
    LocatorQuad lQuad;              // Locator position

    double lMag, lXct, lYct;        // Locator state

    QStack<double> zMag;            // Zoom stack (magnification)
    QStack<double> zXct;            // Zoom stack (X center)
    QStack<double> zYct;            // Zoom stack (Y center)
} DotState;
```

### Frame vs View

**Frame**: Floating-point coordinates in genomic space
```c
typedef struct {
    double x, y;    // Lower-left corner
    double w, h;    // Width and height
} Frame;
```

**View**: Integer coordinates in genomic space
```c
typedef struct {
    int64 x, y;     // Lower-left corner
    int64 w, h;     // Width and height
} View;
```

**Conversion**: Frame allows sub-base-pair precision for smooth zooming

---

## File Format Support

### .1aln (ONE Alignment Format)

**Schema**:
```
# Group line for each alignment
P <num> <num>              # A-read and B-read IDs
L <int> <int> <int> <int>  # abeg aend bbeg bend
Q <int>                    # Quality score (diffs)
T <list:int>               # Trace point differences
```

**Loading Process**:
1. `oneReadLine()` reads line-by-line
2. Parse 'P' lines for sequence identifiers
3. Parse 'L' lines for segment coordinates
4. Store segments in `DotSegment` array
5. Build quad-tree spatial index

### .1seq (ONE Sequence Format)

**Schema**:
```
S <int>                    # Scaffold
g <int> <string>           # Gap specification
C <int> <int> <string>     # Contig: length, scaffold, name
D <dna>                    # DNA sequence (2-bit compressed)
```

**Loading Process**:
1. `Read_GDB()` parses header and scaffold/contig records
2. Sequences remain on disk (EXTERNAL mode)
3. `Load_Sequences()` can bring into memory if needed
4. Base frequencies computed for alignment scoring

---

## Visualization System

### Coordinate System

**Screen Space**: Qt widget pixel coordinates (origin: top-left)
```
(0,0) ────────► X
  │
  │
  ▼ Y
```

**Genomic Space**: Biological sequence coordinates (origin: bottom-left)
```
Y ▲
  │
  │
  (0,0) ────────► X
```

**Transformation Chain**:
1. **Frame → View**: Round to integer coordinates
2. **View → Canvas**: Scale and translate to pixel coordinates
3. **Canvas → Screen**: Apply margins and labels

### Rendering Pipeline

```c
void DotCanvas::paintEvent(QPaintEvent *event) {
    // 1. Setup
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing);

    // 2. Draw k-mer dots (if zoomed in)
    if (frame.w < 1000000 && frame.h < 1000000) {
        Dots *dots = dotplot(plot, kmer, &view);
        // Blit dots directly to raster
        for (int i = 0; i < dots->ahit; i++) {
            setPixel(dots->aplot[i].x, dots->aplot[i].y, color);
        }
    }

    // 3. Draw alignment segments (each layer)
    for (int lay = 0; lay < state->nlays; lay++) {
        if (!state->on[lay]) continue;

        QuadLeaf *list = Plot_Layer(plot, lay, &frame);

        // Draw each segment
        for (int i = 0; i < list->length; i++) {
            DotSegment *seg = &layer->segs[list->idx[i]];

            // Transform to canvas coordinates
            int x1 = frameToCanvasX(seg->abeg);
            int y1 = frameToCanvasY(seg->bbeg);
            int x2 = frameToCanvasX(seg->aend);
            int y2 = frameToCanvasY(seg->bend);

            // Set pen color and thickness
            bool forward = (seg->mark & 0x1);
            QColor color = forward ? state->colorF[lay] : state->colorR[lay];
            painter.setPen(QPen(color, state->thick[lay]));

            // Draw line
            painter.drawLine(x1, y1, x2, y2);
        }

        Free_List(list);
    }

    // 4. Draw focus crosshairs
    if (state->fOn && state->fViz) {
        painter.setPen(QPen(state->fColor, 1));
        painter.drawLine(focus_x, 0, focus_x, height());
        painter.drawLine(0, focus_y, width(), focus_y);
    }

    // 5. Draw locator overview
    if (state->lViz) {
        // Small rectangle showing current view position
        drawLocator(&painter);
    }
}
```

### Quad-Tree Spatial Query

**Structure**:
```
Root (entire genome)
├─ Q0 (top-left quadrant)
│  ├─ Q00
│  ├─ Q01
│  ├─ Q02
│  └─ Q03
├─ Q1 (top-right)
├─ Q2 (bottom-left)
└─ Q3 (bottom-right)
```

**Query Algorithm**:
```c
QuadLeaf *Plot_Layer(DotPlot *plot, int ilay, Frame *query) {
    DotLayer *layer = plot->layers[ilay];
    QuadLeaf *result = allocate_leaf_list();

    query_recursive(layer->qtree, query, result);

    return result;
}

void query_recursive(QuadNode *node, Frame *query, QuadLeaf *result) {
    if (!intersects(node->bounds, query))
        return;

    if (is_leaf(node)) {
        // Add all segments in this leaf
        append_to_result(result, node->segments);
    } else {
        // Recurse to children
        for (int i = 0; i < 4; i++) {
            if (node->quads[i] != NULL)
                query_recursive(node->quads[i], query, result);
        }
    }
}
```

---

## Interaction Model

### Mouse Interactions

| Action | Trigger | Effect |
|--------|---------|--------|
| **Zoom in** | Drag rubber-band rectangle | Zoom to selected region |
| **Pick segment** | Click near segment | Show segment info popup |
| **Pan** | Arrow keys | Shift view by 10% |
| **Zoom +/-** | +/- keys or toolbar | Zoom in/out by 2x |
| **Zoom stack** | 'z' key | Pop previous zoom level |

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `+` | Zoom in 2x |
| `-` | Zoom out 2x |
| `z` | Pop zoom stack |
| `↑↓←→` | Pan view |
| `Space` | Toggle pick mode |
| `Cmd+O` | Open file |
| `Cmd+W` | Close window |

### Focus System

**Purpose**: Highlight specific genomic coordinate

**Activation**:
1. User checks "Focus" checkbox
2. Enters coordinate in focus field
3. Coordinate parsed and validated
4. Crosshairs drawn at position

**Use Cases**:
- Align multiple windows to same coordinate
- Mark region of interest
- Navigate to known positions

---

## Memory Management

### Reference Counting

**Problem**: Multiple windows may share same `DotPlot` data

**Solution**: Reference counting on shared objects
```c
typedef struct {
    int nref;               // Reference count
    // ... other fields
} DotLayer;

typedef struct {
    int nref;               // Reference count
    GDB gdb;
    // ... other fields
} DotGDB;
```

**Lifecycle**:
1. `createPlot()`: Allocates, sets nref=1
2. `copyPlot()`: Shares data structures, increments nref
3. `Free_DotPlot()`: Decrements nref, frees when nref=0

### Memory Pools

**Quad-Tree Blocks**:
- Allocate nodes in 100K blocks (BLK_SIZE)
- Reduces allocation overhead
- Freed together when layer destroyed

**K-mer Dot Plot**:
- Single allocation for dot memory (dotmemory)
- Reused for each dot plot generation
- Max size: MAX_DOTPLOT * sizeof(Tuple)

### Qt Memory Management

- All Qt widgets are children of parent windows
- Qt's object tree handles automatic deletion
- `QList<DotWindow*>` tracks open windows
- Window close event triggers cleanup

---

## Build System

### Project Structure
```
viewer.pro          # Qt project file (qmake)
  ├─ CONFIG += release
  ├─ QMAKE_CXXFLAGS += -DINTERACTIVE
  ├─ QMAKE_CFLAGS += -DINTERACTIVE
  └─ QMAKE_LIBS += -lz

BUILD/              # Build output directory
  ├─ moc_*.cpp      # Qt meta-object compiler output
  ├─ qrc_*.cpp      # Qt resource compiler output
  └─ *.o            # Object files

Makefile            # Generated by qmake
```

### Compilation Flags

**-DINTERACTIVE**:
- Enables interactive error handling mode
- Functions return error codes instead of calling exit()
- Error messages placed in `Ebuffer` (see gene_core.h)

**Key Dependencies**:
- Qt 6.9.0+ (widgets module)
- zlib (for ONE file compression)
- C++11 compiler
- C99 compiler

### Build Process
```bash
qmake viewer.pro    # Generate Makefile from .pro file
make                # Compile C and C++ sources
                    # Link Qt libraries
                    # Produce ALNview executable
```

---

## How It Works: Step-by-Step Walkthrough

### Scenario: User Opens Alignment File and Zooms

#### Step 1: Application Launch
```
main() → QApplication app → DotWindow::openFile() → OpenDialog shown
```

#### Step 2: User Selects File
```
OpenDialog::openALN()
  ├─ Validate file exists
  ├─ Extract filter parameters (length, identity, size)
  └─ Accept dialog
```

#### Step 3: Load Alignment Data
```
sticks.c:createPlot(alnPath, lCut, iCut, sCut)
  ├─ oneFileOpenRead(alnPath)
  │   ├─ Parse ONE file header
  │   ├─ Read schema definition
  │   └─ Initialize decompression codecs
  ├─ Extract genome identifiers from provenance
  ├─ Read_GDB() for both genomes
  │   ├─ Load scaffold structure
  │   ├─ Load contig structure
  │   ├─ Load sequence headers
  │   └─ Keep sequences on disk (EXTERNAL)
  ├─ Read alignment records
  │   ├─ For each ONE record:
  │   │   ├─ Parse coordinates (abeg, aend, bbeg, bend)
  │   │   ├─ Check against filter thresholds
  │   │   └─ If passes: add to segments array
  │   ├─ Allocate DotSegment array
  │   └─ Build quad-tree spatial index
  │       ├─ Recursively partition genome space
  │       ├─ Assign segments to leaf nodes
  │       └─ Store in QuadNode tree
  └─ Return DotPlot structure
```

#### Step 4: Create Main Window
```
new DotWindow(plot, &state, false)
  ├─ Initialize DotState (full genome view)
  ├─ Create DotCanvas
  │   ├─ shareData(state, plot) → link to model
  │   └─ Initialize raster buffers
  ├─ Create toolbar and menus
  ├─ Create layer control panel
  │   ├─ For each layer:
  │   │   ├─ Checkbox (visibility)
  │   │   ├─ Color buttons (forward/reverse)
  │   │   └─ Thickness dropdown
  │   └─ Enable drag-and-drop reordering
  └─ show()
```

#### Step 5: Initial Paint
```
DotCanvas::paintEvent()
  ├─ viewToFrame() → convert view to frame
  ├─ For each visible layer:
  │   ├─ Plot_Layer(plot, i, &frame)
  │   │   ├─ Query quad-tree with frame bounds
  │   │   ├─ Collect intersecting segments
  │   │   └─ Return QuadLeaf list
  │   ├─ For each segment in list:
  │   │   ├─ Transform genomic coords → screen coords
  │   │   └─ QPainter::drawLine(x1, y1, x2, y2)
  │   └─ Free_List()
  └─ Draw axes, labels, focus
```

#### Step 6: User Drags Zoom Rectangle
```
DotCanvas::mousePressEvent(ev)
  ├─ Record start position (xpos, ypos)
  └─ rubber->setGeometry(xpos, ypos, 0, 0)

DotCanvas::mouseMoveEvent(ev)
  └─ rubber->setGeometry(xpos, ypos, dx, dy)

DotCanvas::mouseReleaseEvent(ev)
  ├─ Calculate new frame bounds from rubber-band
  ├─ Push current view onto zoom stack
  ├─ Update frame (x, y, w, h)
  ├─ emit NewFrame(newZoom)
  └─ update() → trigger repaint
```

#### Step 7: Paint Zoomed View
```
DotCanvas::paintEvent()
  ├─ Check if frame.w < 1,000,000 && frame.h < 1,000,000
  │   ├─ YES: Generate k-mer dot plot
  │   │   ├─ doter.c:dotplot(plot, kmer, &view)
  │   │   │   ├─ Extract k-mers from A-sequence in view
  │   │   │   ├─ Hash k-mers → position table
  │   │   │   ├─ Scan B-sequence, find matches
  │   │   │   └─ Return Dots structure
  │   │   └─ Blit dots to raster
  │   └─ NO: Skip k-mer layer
  ├─ Query segments (now zoomed, fewer segments)
  ├─ Draw segments (larger on screen)
  └─ Update coordinate labels
```

#### Step 8: User Picks Segment
```
DotCanvas::mousePressEvent(ev) [with picking enabled]
  ├─ pick(mouse_x, mouse_y)
  │   ├─ Transform screen → genomic coords
  │   ├─ For each visible layer:
  │   │   ├─ Query segments near click point
  │   │   └─ Find closest segment within PICK_LIMIT
  │   └─ Return picked segment + layer
  ├─ popup->show()
  │   ├─ Display segment coordinates
  │   ├─ Display length
  │   └─ Display alignment ID
  └─ Wait for user action

User clicks "Alignment"
  ├─ DotCanvas::showAlign()
  ├─ create_alignment(plot, layer, segment, &title)
  │   ├─ Load trace data from ONE file
  │   ├─ Get_Contig() for A-sequence interval
  │   ├─ Get_Contig() for B-sequence interval
  │   ├─ Setup Alignment structure
  │   ├─ Compute_Trace_PTS() → exact alignment
  │   ├─ Transmit_Alignment() → format as string
  │   └─ Return formatted alignment text
  └─ new AlignWindow(title, atext)
      └─ Display in QTextEdit widget
```

---

## Extensibility Points

### Adding New File Formats
1. Implement loader in `sticks.c:createPlot()`
2. Parse into `DotSegment` array
3. Build quad-tree index
4. No GUI changes needed

### Adding New Coordinate Formats
1. Add format constant to `sticks.h` (FORMAT_*)
2. Implement case in `Map_Coord()` (sticks.c)
3. Add dropdown option in `main_window.cpp`

### Adding New Visualization Layers
1. Create rendering function (similar to `doter.c`)
2. Add layer type to `DotState`
3. Add controls to layer panel
4. Hook into `DotCanvas::paintEvent()`

### Supporting New Alignment Algorithms
1. Extend `align.h` with new trace format
2. Implement compute function in `align.c`
3. Update `create_alignment()` to use new algorithm
4. No GUI changes needed

---

## Performance Considerations

### Spatial Indexing
- Quad-tree reduces segment query from O(n) to O(log n + k)
- Critical for large alignment files (millions of segments)
- Trade-off: build time vs query time

### Lazy Loading
- GDB sequences kept on disk (EXTERNAL mode)
- Only loaded when alignment display requested
- Reduces memory footprint for large genomes

### Rendering Optimization
- Only query segments within current frame
- Cull segments entirely outside viewport
- Use integer math for coordinate transformations
- Limit k-mer dots to prevent memory overflow

### Memory Reuse
- Canvas raster buffer allocated once
- K-mer dot memory pool reused per plot
- Quad-tree nodes allocated in large blocks

---

## Error Handling

### Interactive Mode (-DINTERACTIVE)
- All core functions return error codes
- Error messages placed in `Ebuffer` (global)
- GUI displays errors via `DotWindow::warning()`
- Application continues running

### Common Error Scenarios
- **File not found**: Display error dialog, return to file open
- **Invalid ONE format**: Parse error with line number
- **Memory allocation failure**: Graceful shutdown with message
- **GDB mismatch**: Verify alignment references match loaded genomes

---

## Future Enhancements

### Potential Improvements
1. **GPU acceleration** for rendering many segments
2. **Parallel loading** of alignment files
3. **Save/load session state** (view, layers, colors)
4. **Export rendered image** (PNG, SVG)
5. **Annotation tracks** (genes, features)
6. **Multiple genome comparison** (3-way, N-way)
7. **Search/filter segments** by properties
8. **Statistics panel** (alignment length distribution, identity histogram)

---

## Conclusion

ALNview is a well-architected genomic alignment viewer combining:
- **Efficient C data structures** for performance
- **Modern Qt GUI** for usability
- **Spatial indexing** for scalability
- **Flexible coordinate systems** for navigation
- **Interactive exploration** for analysis

The separation of concerns between data model (C) and presentation (Qt) enables both high performance and rich user experience. The quad-tree indexing and lazy-loading strategies allow the system to handle very large genomic datasets efficiently.
