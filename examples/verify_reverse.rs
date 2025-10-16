use alnview::rust_plot::RustPlot;

fn main() -> anyhow::Result<()> {
    let plot = RustPlot::from_file("/home/erik/sweepga/test.1aln")?;

    println!("Checking reverse complement alignments:\n");

    let mut forward_count = 0;
    let mut reverse_count = 0;

    for (i, seg) in plot.segments.iter().enumerate() {
        if seg.reverse {
            reverse_count += 1;
            if reverse_count <= 5 {
                // For reverse complement, bbeg should be > bend (negative slope)
                let slope_direction = if seg.bbeg > seg.bend { "NEGATIVE ✓" } else { "POSITIVE ✗" };
                println!("Segment {}: REVERSE", i);
                println!("  Query: {} -> {} (len: {})", seg.abeg, seg.aend, seg.aend - seg.abeg);
                println!("  Target: {} -> {} (len: {})", seg.bbeg, seg.bend, seg.bend - seg.bbeg);
                println!("  Slope: {} (bbeg > bend? {})", slope_direction, seg.bbeg > seg.bend);
                println!();
            }
        } else {
            forward_count += 1;
            if forward_count <= 3 {
                // For forward, bbeg should be < bend (positive slope)
                let slope_direction = if seg.bbeg < seg.bend { "POSITIVE ✓" } else { "NEGATIVE ✗" };
                println!("Segment {}: FORWARD", i);
                println!("  Query: {} -> {} (len: {})", seg.abeg, seg.aend, seg.aend - seg.abeg);
                println!("  Target: {} -> {} (len: {})", seg.bbeg, seg.bend, seg.bend - seg.bbeg);
                println!("  Slope: {} (bbeg < bend? {})", slope_direction, seg.bbeg < seg.bend);
                println!();
            }
        }
    }

    println!("Summary:");
    println!("  Forward alignments: {}", forward_count);
    println!("  Reverse alignments: {}", reverse_count);
    println!("  Total: {}", plot.segments.len());

    Ok(())
}
