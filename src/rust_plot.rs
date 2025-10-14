// Pure Rust implementation of plot data structures
use crate::aln_reader::{AlnFile, AlnRecord};
use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct AlignmentSegment {
    pub abeg: i64,
    pub aend: i64,
    pub bbeg: i64,
    pub bend: i64,
    pub reverse: bool,
}

pub struct RustPlot {
    // Genome information
    pub query_sequences: Vec<String>,
    pub target_sequences: Vec<String>,
    pub query_lengths: Vec<i64>,
    pub target_lengths: Vec<i64>,

    // Total genome lengths
    pub query_genome_len: i64,
    pub target_genome_len: i64,

    // Alignment segments (one layer for now)
    pub segments: Vec<AlignmentSegment>,

    // Scaffold boundaries (cumulative positions)
    pub query_boundaries: Vec<i64>,
    pub target_boundaries: Vec<i64>,
}

impl RustPlot {
    /// Load a .1aln file and create plot data
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut aln_file = AlnFile::open(path)?;

        // Read all alignment records
        let records = aln_file.read_all_records()?;

        // Get sequence information (may be empty if file has no names)
        let mut query_sequences = aln_file.query_sequences.clone();
        let mut target_sequences = aln_file.target_sequences.clone();

        // Calculate sequence lengths from the records
        // Use max coordinates seen in alignments. Grow vectors dynamically as needed.
        let mut query_lengths = vec![0i64; query_sequences.len()];
        let mut target_lengths = vec![0i64; target_sequences.len()];

        for rec in &records {
            let qid = rec.query_id as usize;
            let tid = rec.target_id as usize;

            // Grow vectors if needed
            if qid >= query_lengths.len() {
                query_lengths.resize(qid + 1, 0);
            }
            if tid >= target_lengths.len() {
                target_lengths.resize(tid + 1, 0);
            }

            query_lengths[qid] = query_lengths[qid].max(rec.query_end);
            target_lengths[tid] = target_lengths[tid].max(rec.target_end);
        }

        // Generate placeholder names if needed
        while query_sequences.len() < query_lengths.len() {
            let id = query_sequences.len();
            query_sequences.push(format!("query_{}", id));
        }
        while target_sequences.len() < target_lengths.len() {
            let id = target_sequences.len();
            target_sequences.push(format!("target_{}", id));
        }

        // Calculate total genome lengths
        let query_genome_len: i64 = query_lengths.iter().sum();
        let target_genome_len: i64 = target_lengths.iter().sum();

        // Calculate scaffold boundaries (cumulative positions)
        let mut query_boundaries = Vec::new();
        let mut cumulative = 0i64;
        for &len in &query_lengths {
            query_boundaries.push(cumulative);
            cumulative += len;
        }
        query_boundaries.push(cumulative); // Add final boundary

        let mut target_boundaries = Vec::new();
        cumulative = 0;
        for &len in &target_lengths {
            target_boundaries.push(cumulative);
            cumulative += len;
        }
        target_boundaries.push(cumulative); // Add final boundary

        // Now convert records to segments with genome-wide coordinates
        let segments: Vec<AlignmentSegment> = records.iter().enumerate().map(|(i, rec)| {
            let qid = rec.query_id as usize;
            let tid = rec.target_id as usize;

            // Get scaffold offsets
            let query_offset = if qid < query_boundaries.len() {
                query_boundaries[qid]
            } else {
                0
            };
            let target_offset = if tid < target_boundaries.len() {
                target_boundaries[tid]
            } else {
                0
            };

            // For reverse complement: subtract from END of target sequence (like C code)
            // C code: bbeg = (offset + seqlen) - rec.target_start
            let (bbeg, bend) = if rec.reverse != 0 {
                let target_seq_len = if tid < target_lengths.len() {
                    target_lengths[tid]
                } else {
                    0
                };
                let target_end_pos = target_offset + target_seq_len;
                (target_end_pos - rec.target_start, target_end_pos - rec.target_end)
            } else {
                (target_offset + rec.target_start, target_offset + rec.target_end)
            };

            // Convert to genome-wide coordinates
            AlignmentSegment {
                abeg: query_offset + rec.query_start,
                aend: query_offset + rec.query_end,
                bbeg,
                bend,
                reverse: rec.reverse != 0,
            }
        }).collect();

        Ok(Self {
            query_sequences,
            target_sequences,
            query_lengths,
            target_lengths,
            query_genome_len,
            target_genome_len,
            segments,
            query_boundaries,
            target_boundaries,
        })
    }

    /// Get query genome length (A genome)
    pub fn get_alen(&self) -> i64 {
        self.query_genome_len
    }

    /// Get target genome length (B genome)
    pub fn get_blen(&self) -> i64 {
        self.target_genome_len
    }

    /// Get number of layers (always 1 for now)
    pub fn get_nlays(&self) -> i32 {
        1
    }

    /// Get scaffold boundaries for a genome (0 = query, 1 = target)
    pub fn get_scaffold_boundaries(&self, genome: i32) -> Vec<i64> {
        match genome {
            0 => self.query_boundaries.clone(),
            1 => self.target_boundaries.clone(),
            _ => Vec::new(),
        }
    }

    /// Query segments in a visible region
    /// Returns segments that intersect with the region [x, x+width] x [y, y+height]
    pub fn query_segments_in_region(
        &self,
        _layer: i32,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Vec<AlignmentSegment> {
        let x_min = x as i64;
        let x_max = (x + width) as i64;
        let y_min = y as i64;
        let y_max = (y + height) as i64;

        self.segments.iter()
            .filter(|seg| {
                // Check if segment intersects with visible region
                let seg_x_min = seg.abeg.min(seg.aend);
                let seg_x_max = seg.abeg.max(seg.aend);
                let seg_y_min = seg.bbeg.min(seg.bend);
                let seg_y_max = seg.bbeg.max(seg.bend);

                // Intersection test
                seg_x_max >= x_min && seg_x_min <= x_max &&
                seg_y_max >= y_min && seg_y_min <= y_max
            })
            .cloned()
            .collect()
    }
}
