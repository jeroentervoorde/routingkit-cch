use std::{
    env, fs,
    path::{Path, PathBuf},
};

struct OpenMPConfig {
    c_flags: Vec<&'static str>,
    link_libs: Vec<&'static str>,
    link_search: Vec<String>,
    link_args: Vec<String>,
    include_dirs: Vec<String>,
}

fn compute_openmp_config(target: &str, is_msvc: bool, is_clang: bool) -> OpenMPConfig {
    if is_msvc {
        return OpenMPConfig {
            c_flags: vec!["/openmp"],
            link_libs: vec![], // vcomp auto handled by MSVC
            link_search: vec![],
            link_args: vec![],
            include_dirs: vec![],
        };
    }

    // Non-MSVC
    if target.contains("apple-darwin") {
        // macOS: require brew install libomp
        let mut search = vec![];
        let mut includes = vec![];
        let mut extra_link_args = vec![];
        if let Ok(prefix_out) = std::process::Command::new("brew").arg("--prefix").output() {
            if let Ok(prefix) = String::from_utf8(prefix_out.stdout) {
                let p = prefix.trim();
                if !p.is_empty() {
                    search.push(format!("{p}/lib"));
                    includes.push(format!("{p}/include"));
                    extra_link_args.push(format!("-Wl,-rpath,{p}/lib"));
                }
            }
        }
        OpenMPConfig {
            c_flags: vec!["-Xpreprocessor", "-fopenmp"],
            link_libs: vec!["omp"],
            link_search: search,
            link_args: extra_link_args,
            include_dirs: includes,
        }
    } else {
        // Linux / MinGW etc.
        let lib = if is_clang { "omp" } else { "gomp" };
        OpenMPConfig {
            c_flags: vec!["-fopenmp"],
            link_libs: vec![lib],
            link_search: vec![],
            link_args: vec![],
            include_dirs: vec![],
        }
    }
}

fn apply_openmp_config<FAddFlag, FAddInclude>(
    omp: &OpenMPConfig,
    mut add_flag: FAddFlag,
    mut add_include: FAddInclude,
) where
    FAddFlag: FnMut(&str),
    FAddInclude: FnMut(&str),
{
    for f in &omp.c_flags {
        add_flag(f);
    }
    for inc in &omp.include_dirs {
        add_include(inc);
    }
    for s in &omp.link_search {
        println!("cargo:rustc-link-search=native={s}");
    }
    for lib in &omp.link_libs {
        println!("cargo:rustc-link-lib={lib}");
    }
    for arg in &omp.link_args {
        println!("cargo:rustc-link-arg={arg}");
    }
    if let Ok(lib_dir) = env::var("OPENMP_LIB_DIR") {
        println!("cargo:rustc-link-search=native={lib_dir}");
    }
    if let Ok(inc_dir) = env::var("OPENMP_INCLUDE_DIR") {
        add_include(&inc_dir);
    }
}

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
    build.include(&src_dir); // for private headers required by patched files

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
            if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
                if allow.contains(&fname) {
                    if let Some(patcher) =
                        PATCHERS.iter().find(|(n, _)| *n == fname).map(|(_, f)| *f)
                    {
                        let patched = emit_patched_with(&path, patcher)
                            .unwrap_or_else(|e| panic!("failed to patch {fname}: {e}"));
                        build.file(patched);
                    } else {
                        build.file(&path);
                    }
                }
            }
        }
    } else {
        panic!("The RoutingKit src directory does not exist: {src_dir:?}");
    }
    build.file("src/routingkit_cch_wrapper.cc");

    if cfg!(target_env = "msvc") {
        build.define("ROUTING_KIT_NO_GCC_EXTENSIONS", None);
    }

    // Common language standard & optimization
    build.flag_if_supported("-std=c++17");
    build.flag_if_supported("/std:c++17"); // MSVC variant
    build.flag_if_supported("-O3");
    build.flag_if_supported("/O2"); // MSVC approximate

    // Position independent code (shared libs on *nix)
    build.flag_if_supported("-fPIC");

    // Warning level roughly equivalent
    build.flag_if_supported("-Wall");
    build.flag_if_supported("/W4"); // high warning level on MSVC

    // Fast math / floating point contraction
    build.flag_if_supported("-ffast-math"); // GCC/Clang
    build.flag_if_supported("/fp:fast"); // MSVC alternative

    // Native architecture tuning (skip under cross compilation or if unsupported)
    // We avoid passing on MSVC because /arch:native is not a thing; /arch:AVX2 etc would be explicit.
    if !cfg!(target_env = "msvc") {
        build.flag_if_supported("-march=native");
    }

    // Exception handling (MSVC)
    build.flag_if_supported("/EHsc");

    // Disable some noisy warnings
    build.flag_if_supported("-Wno-unused-parameter");
    build.flag_if_supported("-Wno-psabi");
    build.flag_if_supported("-Wno-unused-variable");
    build.flag_if_supported("-Wno-unused-function");

    // OpenMP
    let target = env::var("TARGET").unwrap_or_default();
    let compiler = build.get_compiler();
    let omp = compute_openmp_config(&target, compiler.is_like_msvc(), compiler.is_like_clang());
    {
        use std::cell::RefCell;
        let cell = RefCell::new(&mut build);
        apply_openmp_config(
            &omp,
            |f| {
                cell.borrow_mut().flag(f);
            },
            |inc| {
                cell.borrow_mut().include(inc);
            },
        );
    }

    // -pthread
    build.flag_if_supported("-pthread");

    // Define NDEBUG for release-like (opt) builds; keep assertions in debug.
    if env::var("PROFILE").map(|p| p == "release").unwrap_or(false) {
        build.define("NDEBUG", None);
    } else {
        println!("cargo:warning=Compiling C++ in debug mode.");
    }

    build.compile("routingkit_cch");

    println!("cargo:rerun-if-env-changed=ROUTINGKIT_DIR");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed={}", include_dir.display());
    println!("cargo:rerun-if-changed={}", src_dir.display());
    println!("cargo:rerun-if-changed=src/routingkit_cch_wrapper.h");
    println!("cargo:rerun-if-changed=src/routingkit_cch_wrapper.cc");
}

/// Map of file name -> patch transform function.
/// Each transformer receives original file content and must return new content (or panic/todo!).
static PATCHERS: &[(&str, fn(&str) -> String)] = &[
    (
        "customizable_contraction_hierarchy.cpp",
        patch_customizable_contraction_hierarchy,
    ),
    // Add more (filename, function) pairs here as needed.
];

/// Generic emit helper using provided transformer.
fn emit_patched_with(original: &Path, transform: fn(&str) -> String) -> std::io::Result<PathBuf> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set by cargo"));
    let patched_dir = out_dir.join("patched");
    if !patched_dir.exists() {
        fs::create_dir_all(&patched_dir)?;
    }
    let target = patched_dir.join(original.file_name().unwrap());
    let src = fs::read_to_string(original)?;
    let transformed: String = transform(&src);
    fs::write(&target, transformed)?;
    println!("cargo:warning=Patched {:?}", original.file_name().unwrap());
    Ok(target)
}

fn patch_customizable_contraction_hierarchy(original: &str) -> String {
    let mut lines = original.lines().map(|x| x.to_string()).collect::<Vec<_>>();
    for row in [922, 930] {
        lines[row] = lines[row].replace("unsigned", "long");
    }
    lines.join("\n")
}
