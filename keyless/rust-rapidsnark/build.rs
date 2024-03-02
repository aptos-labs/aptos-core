use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("{}", env::current_dir().expect("Couldn't get working dir.").to_str().expect("couldn't convert pathbuf to str."));


    println!("{:?}", Command::new("bash")
        .arg("-c")
        .arg("cd rapidsnark && ./build_lib.sh")
        .output()
        .expect("Failed to build c++ library"));




    let libdir_path = PathBuf::from("rapidsnark/package/lib")
    // Canonicalize the path as `rustc-link-search` requires an absolute
    // path.
    .canonicalize()
    .expect("cannot canonicalize libdir path");
    let include_path = PathBuf::from("wrapper.hpp")
    // Canonicalize the path as `rustc-link-search` requires an absolute
    // path.
    .canonicalize()
    .expect("cannot canonicalize include path");

    // Tell cargo to tell rustc to link the system `clang`
    // shared library.
    println!("cargo:rerun-if-env-changed=LIBCLANG_PATH");
    println!("cargo:rerun-if-env-changed=LIBCLANG_STATIC_PATH");
    println!("cargo:rerun-if-env-changed=OPENMP_LIBRARY_PATH");

    println!("cargo:rerun-if-env-changed=LIBCLANG_DYNAMIC_PATH");

    println!("cargo:rustc-link-search=native=/usr/lib/llvm-14/lib");


    if let Ok(libclang_path) = env::var("LIBCLANG_PATH") {
        println!("cargo:rustc-link-search=native={}", libclang_path);
    }

    // Specify the C++ standard library
    if let Ok(std_cpp_lib_path) = env::var("CXXSTDLIB_PATH") {
        println!("cargo:rustc-link-search=native={}", std_cpp_lib_path);
    }



    // Tell cargo to look for shared libraries in the specified directory
    println!("cargo:rustc-link-search={}", libdir_path.to_str().unwrap());

    // Tell cargo to tell rustc to link the system bzip2
    // shared library.
    println!("cargo:rustc-link-lib=static=rapidsnark-fr-fq");

    // println!("cargo:rustc-link-lib=c++"); // This is needed on macos
    println!("cargo:rustc-link-lib=stdc++"); // This is needed on linux (will error on macos)
                                
    println!("cargo:rustc-link-lib=dylib=omp");
    println!("cargo:rustc-link-lib=dylib=gomp");
    println!("cargo:rustc-link-lib=dylib=gmp");

    println!("cargo:rustc-link-arg=-fopenmp");




    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header(include_path.to_str().unwrap())
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .clang_arg("-fopenmp")
        .clang_arg("-L/usr/lib/llvm-14/lib")
        .clang_arg("-I./rapidsnark/package/include")
        .clang_arg("-I/usr/lib/llvm-14/lib/clang/14.0.6/include")
        .clang_arg("-I/usr/include/c++/12/")
        .clang_arg("-I/usr/include/x86_64-linux-gnu/c++/12/")
        .clang_arg("-I./rapidsnark/depends/json/single_include")
        .clang_arg("-I./rapidsnark/depends/ffiasm/c")
        .clang_arg("-I./rapidsnark/build")
        .clang_arg("-I./rapidsnark/src")
        .clang_arg("-std=c++17")
        .clang_arg("-stdlib=libc++")
        .blocklist_file("alt_bn128.hpp")
        .blocklist_file("groth16.hpp")
        .blocklist_file("binfile_utils.hpp")
        .blocklist_file("curve.hpp")
        .blocklist_file("zkey_utils.hpp")
        .allowlist_file("fullprover.hpp")
        .allowlist_type("FullProver")
        .allowlist_type("ProverResponseType")
        .allowlist_type("ProverError")
        .allowlist_type("ProverResponseMetrics")

        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
