use alnview::rust_plot::RustPlot;
use std::env;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <file.1aln>", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];
    let plot = RustPlot::from_file(filename)?;

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
