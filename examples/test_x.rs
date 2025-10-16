use alnview::rust_plot::RustPlot;

fn main() -> anyhow::Result<()> {
    println!("Loading x.1aln...");
    let plot = RustPlot::from_file("/home/erik/sweepga/x.1aln")?;

    println!("✓ Loaded successfully!");
    println!("\nPlot information:");
    println!("  Query genome length: {} bp", plot.get_alen());
    println!("  Target genome length: {} bp", plot.get_blen());
    println!(
        "  Number of query sequences: {}",
        plot.query_sequences.len()
    );
    println!(
        "  Number of target sequences: {}",
        plot.target_sequences.len()
    );
    println!("  Number of alignment segments: {}", plot.segments.len());

    println!("\nFirst 5 query sequences:");
    for (i, name) in plot.query_sequences.iter().take(5).enumerate() {
        println!("  {}: {} (len: {})", i, name, plot.query_lengths[i]);
    }

    println!("\nFirst 5 target sequences:");
    for (i, name) in plot.target_sequences.iter().take(5).enumerate() {
        println!("  {}: {} (len: {})", i, name, plot.target_lengths[i]);
    }

    println!("\nFirst 5 alignment segments:");
    for (i, seg) in plot.segments.iter().take(5).enumerate() {
        let dir = if seg.reverse { "REVERSE" } else { "FORWARD" };
        println!(
            "  Segment {}: ({}, {}) -> ({}, {}) [{}]",
            i, seg.abeg, seg.aend, seg.bbeg, seg.bend, dir
        );
    }

    println!("\n✓ All checks passed!");
    Ok(())
}
