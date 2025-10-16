use fastga_rs::AlnReader;

fn main() -> anyhow::Result<()> {
    let mut reader = AlnReader::open("/home/erik/sweepga/x.1aln")?;

    println!("Testing sequence name reading:");

    // Try reading query sequences
    println!("\nQuery sequences:");
    for i in 0..5 {
        match reader.get_seq_name(i, 0) {
            Ok(name) => println!("  Query {}: {}", i, name),
            Err(e) => {
                println!("  Query {}: ERROR - {:?}", i, e);
                break;
            }
        }
    }

    // Try reading target sequences
    println!("\nTarget sequences:");
    for i in 0..5 {
        match reader.get_seq_name(i, 1) {
            Ok(name) => println!("  Target {}: {}", i, name),
            Err(e) => {
                println!("  Target {}: ERROR - {:?}", i, e);
                break;
            }
        }
    }

    // Try reading first record
    println!("\nFirst alignment record:");
    if let Some(rec) = reader.read_record()? {
        println!("  query_id: {}, target_id: {}", rec.query_id, rec.target_id);
        println!(
            "  query_len: {}, target_len: {}",
            rec.query_len, rec.target_len
        );
        println!(
            "  query: {}..{}, target: {}..{}",
            rec.query_start, rec.query_end, rec.target_start, rec.target_end
        );
    } else {
        println!("  No records found");
    }

    Ok(())
}
