//! FFI bindings to C backend
//!
//! These are manual bindings to the existing C code. Eventually we'll replace
//! these modules with safe Rust implementations.

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use rstar::{RTree, RTreeObject, AABB};
use std::os::raw::{c_char, c_int};

// ============================================================================
// Core Data Structures
// ============================================================================

#[repr(C)]
pub struct Frame {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[repr(C)]
pub struct View {
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
}

#[repr(C)]
pub struct Focus {
    pub x: i64,
    pub y: i64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DotSegment {
    pub abeg: i64,
    pub aend: i64,
    pub bbeg: i64,
    pub bend: i64,
    pub iid: i16,
    pub mark: i16,
    pub idx: i32,
}

#[repr(C)]
pub struct QuadLeaf {
    pub length: i32,
    pub depth: i32,
    pub idx: [i32; 8], // This is actually variable-sized, handle with care!
}

// Opaque pointer - don't access struct fields directly from Rust
pub enum DotPlot {}
pub enum DotLayer {}
pub enum DotGDB {}
pub enum GDB {}
pub enum OneFile {}

// ============================================================================
// Sticks (Plot Management)
// ============================================================================

extern "C" {
    /// Create a plot from an alignment file
    /// Returns NULL on error
    pub fn createPlot(
        alnPath: *const c_char,
        lCut: c_int,
        iCut: c_int,
        sCut: c_int,
        plot: *mut DotPlot,
    ) -> *mut DotPlot;

    /// Copy a plot (for multiple windows)
    pub fn copyPlot(plot: *mut DotPlot) -> *mut DotPlot;

    /// Query segments in a layer within a frame
    pub fn Plot_Layer(plot: *mut DotPlot, ilay: c_int, query: *const Frame) -> *mut QuadLeaf;

    /// Free the list returned by Plot_Layer
    pub fn Free_List(list: *mut QuadLeaf);

    /// Free a plot
    pub fn Free_DotPlot(plot: *mut DotPlot);

    /// Accessor functions for DotPlot fields
    pub fn DotPlot_GetAlen(plot: *mut DotPlot) -> i64;
    pub fn DotPlot_GetBlen(plot: *mut DotPlot) -> i64;
    pub fn DotPlot_GetNlays(plot: *mut DotPlot) -> c_int;

    /// Get raw segment array for a layer
    pub fn DotPlot_GetSegments(
        plot: *mut DotPlot,
        layer: c_int,
        count: *mut i64,
    ) -> *const DotSegment;

    /// Get scaffold boundaries (genome: 0=A, 1=B)
    /// Returns array of cumulative positions, caller must free()
    pub fn DotPlot_GetScaffoldBoundaries(
        plot: *mut DotPlot,
        genome: c_int,
        count: *mut c_int,
    ) -> *mut i64;

    /// Create alignment text for a segment
    pub fn create_alignment(
        plot: *mut DotPlot,
        layer: *mut DotLayer,
        seg: *mut DotSegment,
        title: *mut *mut c_char,
    ) -> *mut c_char;

    /// Map coordinate to string
    pub fn Map_Coord(
        gdb: *mut GDB,
        coord1: i64,
        coord2: i64,
        format: c_int,
        width: i64,
    ) -> *mut c_char;
}

// ============================================================================
// Helper Functions
// ============================================================================

impl Frame {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Frame { x, y, w, h }
    }
}

impl View {
    pub fn new(x: i64, y: i64, w: i64, h: i64) -> Self {
        View { x, y, w, h }
    }

    pub fn to_frame(&self) -> Frame {
        Frame {
            x: self.x as f64,
            y: self.y as f64,
            w: self.w as f64,
            h: self.h as f64,
        }
    }
}

impl DotSegment {
    /// Check if segment is reverse complement
    pub fn is_reverse(&self) -> bool {
        (self.mark & 0x1) == 0
    }

    /// Get bounding box for spatial indexing
    pub fn bbox(&self) -> AABB<[f64; 2]> {
        let min_x = self.abeg.min(self.aend) as f64;
        let max_x = self.abeg.max(self.aend) as f64;
        let min_y = self.bbeg.min(self.bend) as f64;
        let max_y = self.bbeg.max(self.bend) as f64;
        AABB::from_corners([min_x, min_y], [max_x, max_y])
    }
}

/// Wrapper for DotSegment that implements RTreeObject for spatial indexing
#[derive(Debug, Clone, Copy)]
pub struct IndexedSegment {
    pub segment: DotSegment,
}

impl RTreeObject for IndexedSegment {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.segment.bbox()
    }
}

impl From<DotSegment> for IndexedSegment {
    fn from(segment: DotSegment) -> Self {
        IndexedSegment { segment }
    }
}

// ============================================================================
// Safe Wrappers
// ============================================================================

/// Safe wrapper around DotPlot pointer with spatial index
pub struct SafePlot {
    ptr: *mut DotPlot,
    /// R*-trees for each layer (indexed by layer number)
    spatial_indices: Vec<RTree<IndexedSegment>>,
}

impl SafePlot {
    /// Create a new SafePlot from a raw pointer
    /// # Safety
    /// The caller must ensure ptr is valid or null
    pub unsafe fn new(ptr: *mut DotPlot) -> Option<Self> {
        if ptr.is_null() {
            return None;
        }

        // Build spatial indices for all layers
        let nlays = unsafe { DotPlot_GetNlays(ptr) };
        let mut spatial_indices = Vec::with_capacity(nlays as usize);

        println!("🌳 Building R*-trees for {nlays} layers...");

        for layer in 0..nlays {
            // Get all segments for this layer
            let segments = unsafe {
                let mut count: i64 = 0;
                let seg_ptr = DotPlot_GetSegments(ptr, layer, &mut count as *mut i64);
                if seg_ptr.is_null() || count == 0 {
                    &[]
                } else {
                    std::slice::from_raw_parts(seg_ptr, count as usize)
                }
            };

            // Convert to indexed segments and build R*-tree
            let indexed: Vec<IndexedSegment> = segments
                .iter()
                .map(|&seg| IndexedSegment::from(seg))
                .collect();

            println!("  Layer {}: {} segments", layer, indexed.len());
            let rtree = RTree::bulk_load(indexed);
            spatial_indices.push(rtree);
        }

        println!("✅ R*-trees built successfully!");

        Some(SafePlot {
            ptr,
            spatial_indices,
        })
    }

    pub fn as_ptr(&self) -> *mut DotPlot {
        self.ptr
    }

    /// Get genome A length
    pub fn get_alen(&self) -> i64 {
        unsafe { DotPlot_GetAlen(self.ptr) }
    }

    /// Get genome B length
    pub fn get_blen(&self) -> i64 {
        unsafe { DotPlot_GetBlen(self.ptr) }
    }

    /// Get number of layers
    pub fn get_nlays(&self) -> i32 {
        unsafe { DotPlot_GetNlays(self.ptr) }
    }

    /// Get all segments for a layer (no spatial indexing, just raw array)
    #[allow(dead_code)]
    pub fn get_all_segments(&self, layer: i32) -> &[DotSegment] {
        unsafe {
            let mut count: i64 = 0;
            let ptr = DotPlot_GetSegments(self.ptr, layer, &mut count as *mut i64);
            if ptr.is_null() || count == 0 {
                &[]
            } else {
                std::slice::from_raw_parts(ptr, count as usize)
            }
        }
    }

    /// Query segments in a rectangular region using R*-tree spatial index
    pub fn query_segments_in_region(
        &self,
        layer: i32,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Vec<DotSegment> {
        if layer < 0 || layer as usize >= self.spatial_indices.len() {
            return Vec::new();
        }

        let query_box = AABB::from_corners([x, y], [x + width, y + height]);

        self.spatial_indices[layer as usize]
            .locate_in_envelope(&query_box)
            .map(|indexed| indexed.segment)
            .collect()
    }

    /// Get scaffold boundaries for genome A or B (0 or 1)
    /// Returns Vec of positions, caller owns the data
    pub fn get_scaffold_boundaries(&self, genome: i32) -> Vec<i64> {
        unsafe {
            let mut count: c_int = 0;
            let ptr = DotPlot_GetScaffoldBoundaries(self.ptr, genome, &mut count as *mut c_int);
            if ptr.is_null() || count == 0 {
                Vec::new()
            } else {
                let slice = std::slice::from_raw_parts(ptr, count as usize);
                let vec = slice.to_vec();
                libc::free(ptr as *mut libc::c_void);
                vec
            }
        }
    }

    /// Query segments in a layer
    pub fn query_layer(&self, layer: i32, frame: &Frame) -> Option<SegmentList> {
        unsafe {
            let leaf = Plot_Layer(self.ptr, layer, frame as *const Frame);
            if leaf.is_null() {
                None
            } else {
                Some(SegmentList { ptr: leaf })
            }
        }
    }
}

impl Drop for SafePlot {
    fn drop(&mut self) {
        unsafe {
            Free_DotPlot(self.ptr);
        }
    }
}

unsafe impl Send for SafePlot {}
unsafe impl Sync for SafePlot {}

/// Safe wrapper around QuadLeaf segment list
pub struct SegmentList {
    ptr: *mut QuadLeaf,
}

impl SegmentList {
    pub fn len(&self) -> usize {
        unsafe { (*self.ptr).length as usize }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get segment indices (be careful, this is the C API quirk)
    pub fn indices(&self) -> &[i32] {
        unsafe {
            let len = (*self.ptr).length as usize;
            // WARNING: This is simplified. Real QuadLeaf can have more than 8 indices.
            // We need to handle the variable-sized array properly.
            std::slice::from_raw_parts((*self.ptr).idx.as_ptr(), len.min(8))
        }
    }
}

impl Drop for SegmentList {
    fn drop(&mut self) {
        unsafe {
            Free_List(self.ptr);
        }
    }
}
