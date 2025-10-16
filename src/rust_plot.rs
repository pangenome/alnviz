// Pure Rust implementation of plot data structures
use crate::aln_reader::{AlnFile, AlnRecord};
use crate::sequence_filter::SequenceFilter;
use anyhow::Result;
use std::path::Path;
use std::collections::HashSet;

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

    /// Apply sequence filters to create a subset view
    /// Returns a new RustPlot with only segments involving selected sequences
    pub fn with_filters(&self, query_filter: &SequenceFilter, target_filter: &SequenceFilter) -> Result<Self> {
        // Get matching sequence indices
        let query_indices = query_filter.matching_indices(&self.query_sequences);
        let target_indices = target_filter.matching_indices(&self.target_sequences);

        // If both filters are empty, return clone
        if query_indices.len() == self.query_sequences.len() &&
           target_indices.len() == self.target_sequences.len() {
            return Ok(self.clone());
        }

        // Filter and re-index sequences
        let mut new_query_sequences = Vec::new();
        let mut new_query_lengths = Vec::new();
        let mut old_to_new_query: Vec<Option<usize>> = vec![None; self.query_sequences.len()];

        for (old_idx, name) in self.query_sequences.iter().enumerate() {
            if query_indices.contains(&old_idx) {
                let new_idx = new_query_sequences.len();
                old_to_new_query[old_idx] = Some(new_idx);
                new_query_sequences.push(name.clone());
                new_query_lengths.push(self.query_lengths[old_idx]);
            }
        }

        let mut new_target_sequences = Vec::new();
        let mut new_target_lengths = Vec::new();
        let mut old_to_new_target: Vec<Option<usize>> = vec![None; self.target_sequences.len()];

        for (old_idx, name) in self.target_sequences.iter().enumerate() {
            if target_indices.contains(&old_idx) {
                let new_idx = new_target_sequences.len();
                old_to_new_target[old_idx] = Some(new_idx);
                new_target_sequences.push(name.clone());
                new_target_lengths.push(self.target_lengths[old_idx]);
            }
        }

        // Recalculate boundaries for filtered sequences
        let mut new_query_boundaries = Vec::new();
        let mut cumulative = 0i64;
        for &len in &new_query_lengths {
            new_query_boundaries.push(cumulative);
            cumulative += len;
        }
        new_query_boundaries.push(cumulative);
        let new_query_genome_len = cumulative;

        let mut new_target_boundaries = Vec::new();
        cumulative = 0;
        for &len in &new_target_lengths {
            new_target_boundaries.push(cumulative);
            cumulative += len;
        }
        new_target_boundaries.push(cumulative);
        let new_target_genome_len = cumulative;

        // Filter and re-map segments
        // We need to remap coordinates to the new filtered coordinate system
        let mut new_segments = Vec::new();

        for seg in &self.segments {
            // Find which sequence this segment belongs to
            let query_idx = self.find_sequence_index(&self.query_boundaries, seg.abeg);
            let target_idx = self.find_sequence_index(&self.target_boundaries, seg.bbeg.min(seg.bend));

            // Check if both sequences are in our filter
            if let (Some(new_qidx), Some(new_tidx)) = (
                old_to_new_query.get(query_idx).and_then(|&x| x),
                old_to_new_target.get(target_idx).and_then(|&x| x),
            ) {
                // Remap coordinates to new coordinate system
                let old_q_offset = self.query_boundaries[query_idx];
                let new_q_offset = new_query_boundaries[new_qidx];
                let q_delta = new_q_offset - old_q_offset;

                let old_t_offset = self.target_boundaries[target_idx];
                let new_t_offset = new_target_boundaries[new_tidx];
                let t_delta = new_t_offset - old_t_offset;

                new_segments.push(AlignmentSegment {
                    abeg: seg.abeg + q_delta,
                    aend: seg.aend + q_delta,
                    bbeg: seg.bbeg + t_delta,
                    bend: seg.bend + t_delta,
                    reverse: seg.reverse,
                });
            }
        }

        Ok(Self {
            query_sequences: new_query_sequences,
            target_sequences: new_target_sequences,
            query_lengths: new_query_lengths,
            target_lengths: new_target_lengths,
            query_genome_len: new_query_genome_len,
            target_genome_len: new_target_genome_len,
            segments: new_segments,
            query_boundaries: new_query_boundaries,
            target_boundaries: new_target_boundaries,
        })
    }

    /// Find which sequence a genome coordinate belongs to
    fn find_sequence_index(&self, boundaries: &[i64], coord: i64) -> usize {
        for i in 0..boundaries.len().saturating_sub(1) {
            if coord >= boundaries[i] && coord < boundaries[i + 1] {
                return i;
            }
        }
        boundaries.len().saturating_sub(2).max(0)
    }
}

impl Clone for RustPlot {
    fn clone(&self) -> Self {
        Self {
            query_sequences: self.query_sequences.clone(),
            target_sequences: self.target_sequences.clone(),
            query_lengths: self.query_lengths.clone(),
            target_lengths: self.target_lengths.clone(),
            query_genome_len: self.query_genome_len,
            target_genome_len: self.target_genome_len,
            segments: self.segments.clone(),
            query_boundaries: self.query_boundaries.clone(),
            target_boundaries: self.target_boundaries.clone(),
        }
    }
}
