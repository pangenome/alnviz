// Sequence filtering for subset views
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct SequenceFilter {
    /// Selected sequence names (exact or prefix match)
    pub names: Vec<String>,
    /// Selected sequence index range (inclusive)
    pub range: Option<(usize, usize)>,
}

impl SequenceFilter {
    pub fn new() -> Self {
        Self {
            names: Vec::new(),
            range: None,
        }
    }

    /// Create from comma-separated names/prefixes
    pub fn from_names(names_str: &str) -> Self {
        let names: Vec<String> = names_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Self { names, range: None }
    }

    /// Create from range string like "0-5" or "3-10"
    pub fn from_range(range_str: &str) -> anyhow::Result<Self> {
        let parts: Vec<&str> = range_str.split('-').collect();
        if parts.len() != 2 {
            anyhow::bail!("Range must be in format 'start-end', got: {range_str}");
        }

        let start: usize = parts[0].trim().parse()?;
        let end: usize = parts[1].trim().parse()?;

        if start > end {
            anyhow::bail!("Range start must be <= end");
        }

        Ok(Self {
            names: Vec::new(),
            range: Some((start, end)),
        })
    }

    /// Check if this filter matches any sequences
    pub fn is_empty(&self) -> bool {
        self.names.is_empty() && self.range.is_none()
    }

    /// Check if a sequence at given index with given name matches this filter
    pub fn matches(&self, index: usize, name: &str) -> bool {
        if self.is_empty() {
            return true; // No filter = match all
        }

        // Check range filter
        if let Some((start, end)) = self.range {
            if index >= start && index <= end {
                return true;
            }
        }

        // Check name/prefix filter
        for filter_name in &self.names {
            if name == filter_name || name.starts_with(filter_name) {
                return true;
            }
        }

        false
    }

    /// Get set of matching sequence indices from a list of sequence names
    pub fn matching_indices(&self, sequences: &[String]) -> HashSet<usize> {
        sequences
            .iter()
            .enumerate()
            .filter(|(idx, name)| self.matches(*idx, name))
            .map(|(idx, _)| idx)
            .collect()
    }
}

impl Default for SequenceFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_filter_matches_all() {
        let filter = SequenceFilter::new();
        assert!(filter.matches(0, "chr1"));
        assert!(filter.matches(5, "scaffold_10"));
    }

    #[test]
    fn test_exact_name_match() {
        let filter = SequenceFilter::from_names("chr1,chr2");
        assert!(filter.matches(0, "chr1"));
        assert!(filter.matches(1, "chr2"));
        assert!(!filter.matches(2, "chr3"));
    }

    #[test]
    fn test_prefix_match() {
        let filter = SequenceFilter::from_names("chr");
        assert!(filter.matches(0, "chr1"));
        assert!(filter.matches(1, "chr2_scaffold"));
        assert!(!filter.matches(2, "scaffold_1"));
    }

    #[test]
    fn test_range_filter() {
        let filter = SequenceFilter::from_range("2-5").unwrap();
        assert!(!filter.matches(1, "any"));
        assert!(filter.matches(2, "any"));
        assert!(filter.matches(5, "any"));
        assert!(!filter.matches(6, "any"));
    }

    #[test]
    fn test_combined_filters() {
        let mut filter = SequenceFilter::from_names("chr1");
        filter.range = Some((0, 10));

        // Matches if either name OR range matches
        assert!(filter.matches(0, "chr1")); // matches name
        assert!(filter.matches(5, "scaffold")); // matches range
        assert!(!filter.matches(15, "scaffold")); // matches neither
    }
}
