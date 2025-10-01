use std::env;

fn main() {
    let mut build = cc::Build::new();

    // Core C files
    build
        .file("sticks.c")
        .file("GDB.c")
        .file("ONElib.c")
        .file("align.c")
        .file("gene_core.c")
        .file("hash.c")
        .file("doter.c")
        .file("select.c")
        .file("alncode.c");

    // Compiler flags
    build
        .flag("-DINTERACTIVE")
        .flag("-Wno-unused-result")
        .flag("-Wno-unused-variable")
        .flag("-Wno-unused-function");

    // Enable ASAN in debug mode for bug hunting
    if cfg!(debug_assertions) && env::var("ASAN").is_ok() {
        build
            .flag("-fsanitize=address")
            .flag("-fsanitize=undefined")
            .flag("-g");

        println!("cargo:rustc-link-arg=-fsanitize=address");
        println!("cargo:rustc-link-arg=-fsanitize=undefined");
    }

    // Link zlib
    println!("cargo:rustc-link-lib=z");

    // Compile
    build.compile("alnview_c");

    // Rebuild if C files change
    println!("cargo:rerun-if-changed=sticks.c");
    println!("cargo:rerun-if-changed=sticks.h");
    println!("cargo:rerun-if-changed=GDB.c");
    println!("cargo:rerun-if-changed=GDB.h");
    println!("cargo:rerun-if-changed=ONElib.c");
    println!("cargo:rerun-if-changed=ONElib.h");
    println!("cargo:rerun-if-changed=align.c");
    println!("cargo:rerun-if-changed=align.h");
    println!("cargo:rerun-if-changed=gene_core.c");
    println!("cargo:rerun-if-changed=gene_core.h");
    println!("cargo:rerun-if-changed=hash.c");
    println!("cargo:rerun-if-changed=hash.h");
    println!("cargo:rerun-if-changed=doter.c");
    println!("cargo:rerun-if-changed=doter.h");
    println!("cargo:rerun-if-changed=select.c");
    println!("cargo:rerun-if-changed=select.h");
    println!("cargo:rerun-if-changed=alncode.c");
    println!("cargo:rerun-if-changed=alncode.h");
}
