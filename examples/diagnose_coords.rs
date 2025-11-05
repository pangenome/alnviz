use alnviz::rust_plot::RustPlot;

fn main() -> anyhow::Result<()> {
    println!("PAF Input Analysis:");
    println!("Line 1: query1 1000 0 100 - target1 1000 900 1000");
    println!("  Meaning: Query[0:100] aligns to REVERSE COMPLEMENT of Target[900:1000]");
    println!("  In RC coords: Query 0 aligns with Target RC position 0");
    println!("                which is Target FORWARD position 1000");
    println!("                Query 100 aligns with Target RC position 100");
    println!("                which is Target FORWARD position 900");
    println!("  Expected plot point: (0, 1000) -> (100, 900)\n");

    println!("Line 2: query1 1000 200 300 - target1 1000 700 800");
    println!("  Expected plot point: (200, 800) -> (300, 700)\n");

    println!("If these alignments form a continuous reverse match,");
    println!("they should form a smooth line from top-left to bottom-right\n");

    println!("---\n");

    let plot = RustPlot::from_paf("test_reverse.paf")?;

    println!("ACTUAL coordinates produced by current code:\n");

    for (i, seg) in plot.segments.iter().enumerate() {
        println!("Segment {}: ({}, {}) -> ({}, {})",
            i + 1, seg.abeg, seg.bbeg, seg.aend, seg.bend);
    }

    println!("\n❌ Notice: The actual coordinates are:");
    println!("  Segment 1: (0, 100) -> (100, 0)");
    println!("  Segment 2: (200, 300) -> (300, 200)");
    println!("\nThese are at the WRONG end of the target!");
    println!("They should be near position 1000, not position 0-300");

    Ok(())
}
