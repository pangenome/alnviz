// Module for reading .1aln files using fastga-rs
use anyhow::{Context, Result};
use fastga_rs::AlnReader;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct AlnRecord {
    pub query_id: i64,
    pub target_id: i64,
    #[allow(dead_code)]
    pub query_name: String,
    #[allow(dead_code)]
    pub target_name: String,
    #[allow(dead_code)]
    pub query_len: i64,
    #[allow(dead_code)]
    pub target_len: i64,
    pub query_start: i64,
    pub query_end: i64,
    pub target_start: i64,
    pub target_end: i64,
    pub reverse: i32,
    pub diffs: i32,
}

pub struct AlnFile {
    reader: AlnReader,
    pub query_sequences: Vec<String>,
    pub target_sequences: Vec<String>,
}

impl AlnFile {
    /// Open a .1aln file for reading
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let mut reader = AlnReader::open(path)
            .with_context(|| format!("Failed to open .1aln file: {}", path.display()))?;

        // Read sequence names
        let query_sequences = Self::read_seq_names(&mut reader, 0)?;
        let target_sequences = Self::read_seq_names(&mut reader, 1)?;

        Ok(Self {
            reader,
            query_sequences,
            target_sequences,
        })
    }

    /// Read all sequence names for a given genome (0 = query, 1 = target)
    /// Returns empty Vec if no names are stored in the file
    fn read_seq_names(reader: &mut AlnReader, genome: i32) -> Result<Vec<String>> {
        let mut names = Vec::new();
        let mut seq_id = 0;

        while let Ok(name) = reader.get_seq_name(seq_id, genome) {
            names.push(name);
            seq_id += 1;
        }

        Ok(names)
    }

    /// Get or generate a sequence name for the given ID
    fn get_seq_name(&self, seq_id: i64, genome: i32, sequences: &[String]) -> String {
        if let Some(name) = sequences.get(seq_id as usize) {
            name.clone()
        } else {
            // Generate placeholder name if not in cache
            let prefix = if genome == 0 { "query" } else { "target" };
            format!("{prefix}_{seq_id}")
        }
    }

    /// Read next alignment record
    pub fn read_record(&mut self) -> Result<Option<AlnRecord>> {
        match self.reader.read_record()? {
            Some(rec) => {
                // Get names from cached list or generate placeholder names
                let query_name = self.get_seq_name(rec.query_id, 0, &self.query_sequences);
                let target_name = self.get_seq_name(rec.target_id, 1, &self.target_sequences);

                Ok(Some(AlnRecord {
                    query_id: rec.query_id,
                    target_id: rec.target_id,
                    query_name,
                    target_name,
                    query_len: rec.query_len,
                    target_len: rec.target_len,
                    query_start: rec.query_start,
                    query_end: rec.query_end,
                    target_start: rec.target_start,
                    target_end: rec.target_end,
                    reverse: rec.reverse,
                    diffs: rec.diffs,
                }))
            }
            None => Ok(None),
        }
    }

    /// Get total query genome length (sum of all query sequences)
    #[allow(dead_code)]
    pub fn get_query_genome_len(&self) -> u64 {
        self.query_sequences.len() as u64
    }

    /// Get total target genome length (sum of all target sequences)
    #[allow(dead_code)]
    pub fn get_target_genome_len(&self) -> u64 {
        self.target_sequences.len() as u64
    }

    /// Read all records into a vector
    pub fn read_all_records(&mut self) -> Result<Vec<AlnRecord>> {
        let mut records = Vec::new();
        while let Some(rec) = self.read_record()? {
            records.push(rec);
        }
        Ok(records)
    }
}

/// Calculate identity for an alignment record
pub fn calculate_identity(rec: &AlnRecord) -> f64 {
    let aln_len = (rec.query_end - rec.query_start) as f64;
    if aln_len == 0.0 {
        return 0.0;
    }
    let matches = aln_len - rec.diffs as f64;
    100.0 * matches / aln_len
}
