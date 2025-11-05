use anyhow::{anyhow, Context, Result};
use flate2::read::MultiGzDecoder;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PafRecord {
    pub query_name: String,
    pub query_len: i64,
    pub query_start: i64,
    pub query_end: i64,
    pub strand: char, // '+' or '-'
    pub target_name: String,
    pub target_len: i64,
    pub target_start: i64,
    pub target_end: i64,
    pub matches: i64,
    pub aln_len: i64,
}

/// Parse a PAF file (minimap2 format) into records.
pub fn read_paf_file<P: AsRef<Path>>(path: P) -> Result<Vec<PafRecord>> {
    let path = path.as_ref();
    let mut records = Vec::new();

    // Choose reader: plain text or gzipped
    let file = File::open(path).with_context(|| format!("Failed to open PAF file: {}", path.display()))?;
    let lower = path.to_string_lossy().to_ascii_lowercase();
    let is_bgzip = lower.ends_with(".gz") || lower.ends_with(".bgz") || lower.ends_with(".bgzf");

    if is_bgzip {
        // MultiGzDecoder supports concatenated gzip members (BGZF/BGZIP-compatible)
        let gz = MultiGzDecoder::new(file);
        let reader = BufReader::new(gz);
        parse_paf_from_reader(reader, &mut records)?;
    } else {
        let reader = BufReader::new(file);
        parse_paf_from_reader(reader, &mut records)?;
    }

    Ok(records)
}

fn parse_paf_from_reader<R: Read>(reader: R, out: &mut Vec<PafRecord>) -> Result<()> {
    let mut reader = BufReader::new(reader);
    for (lineno, line_res) in reader.by_ref().lines().enumerate() {
        let line = line_res.with_context(|| format!("Failed to read line {}", lineno + 1))?;
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 12 {
            return Err(anyhow!("Invalid PAF at line {}: expected >=12 fields, got {}", lineno + 1, cols.len()));
        }
        let qname = cols[0].to_string();
        let qlen: i64 = cols[1].parse().with_context(|| format!("line {}: qlen", lineno + 1))?;
        let qstart: i64 = cols[2].parse().with_context(|| format!("line {}: qstart", lineno + 1))?;
        let qend: i64 = cols[3].parse().with_context(|| format!("line {}: qend", lineno + 1))?;
        let strand = cols[4].chars().next().ok_or_else(|| anyhow!("line {}: empty strand", lineno + 1))?;
        if strand != '+' && strand != '-' {
            return Err(anyhow!("line {}: invalid strand '{}'", lineno + 1, strand));
        }
        let tname = cols[5].to_string();
        let tlen: i64 = cols[6].parse().with_context(|| format!("line {}: tlen", lineno + 1))?;
        let tstart: i64 = cols[7].parse().with_context(|| format!("line {}: tstart", lineno + 1))?;
        let tend: i64 = cols[8].parse().with_context(|| format!("line {}: tend", lineno + 1))?;
        let matches: i64 = cols[9].parse().with_context(|| format!("line {}: matches", lineno + 1))?;
        let aln_len: i64 = cols[10].parse().with_context(|| format!("line {}: aln_len", lineno + 1))?;

        out.push(PafRecord {
            query_name: qname,
            query_len: qlen,
            query_start: qstart,
            query_end: qend,
            strand,
            target_name: tname,
            target_len: tlen,
            target_start: tstart,
            target_end: tend,
            matches,
            aln_len,
        });
    }

    Ok(())
}
