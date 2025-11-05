use alnviz::rust_plot::RustPlot;

fn main() -> anyhow::Result<()> {
    let plot = RustPlot::from_paf("test_reverse.paf")?;

    println!("Testing PAF reverse complement handling:\n");
    println!("Expected: All 5 segments should have NEGATIVE slope (bbeg > bend)\n");

    for (i, seg) in plot.segments.iter().enumerate() {
        let slope_direction = if seg.bbeg > seg.bend {
            "NEGATIVE ✓"
        } else if seg.bbeg < seg.bend {
            "POSITIVE ✗ ERROR!"
        } else {
            "HORIZONTAL ✗ ERROR!"
        };

        println!("Segment {}:", i + 1);
        println!("  Query:  {} -> {} (span: {})", seg.abeg, seg.aend, seg.aend - seg.abeg);
        println!("  Target: {} -> {} (span: {})", seg.bbeg, seg.bend, seg.bend - seg.bbeg);
        println!("  Reverse flag: {}", seg.reverse);
        println!("  Slope: {}", slope_direction);
        println!();
    }

    // Check if all have negative slope
    let all_negative = plot.segments.iter().all(|seg| seg.bbeg > seg.bend);
    let all_reverse_flag = plot.segments.iter().all(|seg| seg.reverse);

    println!("Summary:");
    println!("  All segments have negative slope: {}", if all_negative { "✓" } else { "✗ FAIL" });
    println!("  All segments have reverse flag set: {}", if all_reverse_flag { "✓" } else { "✗ FAIL" });

    if !all_negative || !all_reverse_flag {
        println!("\n❌ REVERSE COMPLEMENT HANDLING IS BROKEN!");
        std::process::exit(1);
    } else {
        println!("\n✓ Reverse complement handling is correct");
    }

    Ok(())
}
