extern crate bindgen;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use cmake::Config;

const FASTER_REPO_URL: &str = "https://github.com/zablotchi/FASTER.git";
const FASTER_BRANCH: &str = "main";
const FASTER_DIR: &str = "FASTER";

/// Ensures FASTER source code is available by cloning or updating the repository
fn ensure_faster_source() {
    let faster_path = Path::new(FASTER_DIR);
    let no_update = env::var("FASTER_NO_UPDATE").is_ok();

    if !faster_path.exists() {
        // Clone FASTER repository
        println!("cargo:warning=FASTER source not found, cloning from {}", FASTER_REPO_URL);
        let status = Command::new("git")
            .args(&[
                "clone",
                "--depth", "1",
                "--branch", FASTER_BRANCH,
                FASTER_REPO_URL,
                FASTER_DIR,
            ])
            .status()
            .expect("Failed to execute git clone. Is git installed?");

        if !status.success() {
            panic!("Failed to clone FASTER repository from {}", FASTER_REPO_URL);
        }
        println!("cargo:warning=Successfully cloned FASTER repository");
    } else if !no_update {
        // Update existing FASTER repository to latest
        println!("cargo:warning=Updating FASTER source to latest commit from {}", FASTER_BRANCH);
        let status = Command::new("git")
            .args(&["-C", FASTER_DIR, "pull", "origin", FASTER_BRANCH])
            .status()
            .expect("Failed to execute git pull");

        if !status.success() {
            println!("cargo:warning=Failed to update FASTER repository, using existing version");
        } else {
            println!("cargo:warning=Successfully updated FASTER repository");
        }
    } else {
        println!("cargo:warning=FASTER_NO_UPDATE is set, skipping git pull");
    }

    // Verify directory is not empty
    if fs::read_dir(FASTER_DIR).unwrap().count() == 0 {
        panic!("The `{}` directory is empty after clone/update attempt", FASTER_DIR);
    }
}

fn faster_bindgen() {
    let bindings = bindgen::Builder::default()
        .header("FASTER/cc/src/core/faster-c.h")
        .blocklist_type("max_align_t") // https://github.com/rust-lang-nursery/rust-bindgen/issues/550
        .ctypes_prefix("libc")
        .generate()
        .expect("unable to generate faster bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("unable to write faster bindings");
}


fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=FASTER/");

    ensure_faster_source();

    faster_bindgen();

    let dst = Config::new("FASTER/cc")
        .cflag("--std=c++11 ")
        .build();

    println!("cargo:rustc-link-search=native={}/{}", dst.display(), "build");
    // Fix this...
    println!("cargo:rustc-link-lib=static=faster");
    println!("cargo:rustc-link-lib=stdc++fs");
    println!("cargo:rustc-link-lib=uuid");
    println!("cargo:rustc-link-lib=tbb");
    println!("cargo:rustc-link-lib=gcc");
    println!("cargo:rustc-link-lib=stdc++");
    println!("cargo:rustc-link-lib=aio");
    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rustc-link-lib=m");
}
