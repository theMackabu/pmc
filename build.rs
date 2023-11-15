use chrono::Datelike;
use std::env;
use std::process::Command;

fn main() {
    /* version attributes */
    let date = chrono::Utc::now();
    let profile = env::var("PROFILE").unwrap();
    let output = Command::new("git").args(&["rev-parse", "--short=10", "HEAD"]).output().unwrap();
    let output_full = Command::new("git").args(&["rev-parse", "HEAD"]).output().unwrap();

    println!("cargo:rustc-env=TARGET={}", env::var("TARGET").unwrap());
    println!("cargo:rustc-env=GIT_HASH={}", String::from_utf8(output.stdout).unwrap());
    println!("cargo:rustc-env=GIT_HASH_FULL={}", String::from_utf8(output_full.stdout).unwrap());
    println!("cargo:rustc-env=BUILD_DATE={}-{}-{}", date.year(), date.month(), date.day());

    /* profile matching */
    match profile.as_str() {
        "debug" => println!("cargo:rustc-env=PROFILE=debug"),
        "release" => println!("cargo:rustc-env=PROFILE=release"),
        _ => println!("cargo:rustc-env=PROFILE=none"),
    }

    /* cc linking */
    cxx_build::bridge("src/main.rs")
        .file("src/cc/bridge.cc")
        .file("src/cc/process.cc")
        .flag_if_supported("-std=c++14")
        .compile("bridge");

    let watched = vec!["main.rs", "cc/bridge.cc", "cc/process.cc", "include/process.h"];
    watched.iter().for_each(|file| println!("cargo:rerun-if-changed=src/{}", file));
}
