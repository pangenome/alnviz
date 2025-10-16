use alnview::rust_plot::RustPlot;

fn main() -> anyhow::Result<()> {
    let plot = RustPlot::from_file("test.1aln")?;

    println!("Query sequences ({}):", plot.query_sequences.len());
    for (i, name) in plot.query_sequences.iter().enumerate() {
        println!("  {}: {}", i, name);
    }

    println!("\nTarget sequences ({}):", plot.target_sequences.len());
    for (i, name) in plot.target_sequences.iter().enumerate() {
        println!("  {}: {}", i, name);
    }

    Ok(())
}
