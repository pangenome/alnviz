use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Test that rendering produces consistent output
#[test]
fn test_render_test_1aln_matches_golden() {
    let test_file = PathBuf::from("test.1aln");
    if !test_file.exists() {
        eprintln!("Warning: test.1aln not found, skipping test");
        return;
    }

    let output_path = PathBuf::from("/tmp/test_render.png");
    let golden_path = PathBuf::from("tests/golden/test.1aln.png");

    // Run alnview to render the plot
    let status = Command::new("cargo")
        .args(&[
            "run",
            "--release",
            "--",
            "test.1aln",
            "--plot",
            output_path.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to run alnview");

    assert!(status.success(), "alnview command failed");
    assert!(output_path.exists(), "Output PNG was not created");

    // Check if golden file exists
    if !golden_path.exists() {
        // Generate golden file if it doesn't exist
        eprintln!("Golden file not found, creating: {}", golden_path.display());
        fs::copy(&output_path, &golden_path).expect("Failed to create golden file");
        eprintln!("✅ Golden file created. Please commit it.");
        return;
    }

    // Compare checksums
    let output_data = fs::read(&output_path).expect("Failed to read output file");
    let golden_data = fs::read(&golden_path).expect("Failed to read golden file");

    let output_hash = sha256_digest(&output_data);
    let golden_hash = sha256_digest(&golden_data);

    assert_eq!(
        output_hash, golden_hash,
        "Rendered output doesn't match golden file!\n  Output: {}\n  Golden: {}",
        output_hash, golden_hash
    );

    // Clean up
    fs::remove_file(output_path).ok();
}

/// SHA-256 hash function
fn sha256_digest(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}
