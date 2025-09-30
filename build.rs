use std::{env, path::PathBuf};

fn main() {
    let rk_dir = env::var("ROUTINGKIT_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./RoutingKit"));

    let include_dir = rk_dir.join("include");
    let src_dir = rk_dir.join("src");

    if !include_dir.exists() {
        panic!(
            "The RoutingKit include directory does not exist: {include_dir:?}. 
            Please set ROUTINGKIT_DIR or place it in ./RoutingKit"
        );
    }

    let mut build = cxx_build::bridge("src/lib.rs");
    build.include(&include_dir);
    build.include("src"); // for our wrapper header

    let allow = [
        // Core CCH
        "customizable_contraction_hierarchy.cpp",
        "contraction_hierarchy.cpp", // base CH structures used internally
        // Utilities required by unresolved symbols
        "bit_vector.cpp",        // BitVector
        "bit_select.cpp",        // popcount helpers
        "id_mapper.cpp",         // LocalIDMapper
        "permutation.cpp", // compute_*_sort_permutation (actually part of nested_dissection or permutation utilities)
        "nested_dissection.cpp", // may be referenced by order related helpers
        "verify.cpp",      // check_contraction_hierarchy_for_errors
        "graph_util.cpp",  // find_arc_given_sorted_head
        "timer.cpp",       // get_micro_time
                           // Keep minimal; add more if linker still complains
    ];
    if src_dir.exists() {
        for entry in std::fs::read_dir(&src_dir).unwrap() {
            let path = entry.unwrap().path();
            if allow.contains(&path.file_name().unwrap().to_str().unwrap()) {
                build.file(&path);
            }
        }
    } else {
        panic!("The RoutingKit src directory does not exist: {src_dir:?}");
    }
    build.file("src/routingkit_cch_wrapper.cc");

    if cfg!(target_env = "msvc") {
        build.define("ROUTING_KIT_NO_GCC_EXTENSIONS", None);
    }

    build.flag_if_supported("-std=c++17");
    build.flag_if_supported("/std:c++17"); // MSVC
    build.flag_if_supported("-O3");
    build.flag_if_supported("/O2"); // MSVC
    build.flag_if_supported("/EHsc");
    build.flag_if_supported("-Wno-unused-parameter");
    build.flag_if_supported("-Wno-psabi");
    build.flag_if_supported("-Wno-unused-variable");
    build.flag_if_supported("-Wno-unused-function");

    build.compile("routingkit_cch");

    println!("cargo:rerun-if-env-changed=ROUTINGKIT_DIR");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed={}", include_dir.display());
    println!("cargo:rerun-if-changed={}", src_dir.display());
    println!("cargo:rerun-if-changed=src/routingkit_cch_wrapper.h");
    println!("cargo:rerun-if-changed=src/routingkit_cch_wrapper.cc");
}
