fn main() {
    cxx_build::bridge("src/main.rs")
        .file("src/cc/bridge.cc")
        .file("src/cc/process.cc")
        .flag_if_supported("-std=c++14")
        .compile("bridge");

    let watched = vec!["main.rs", "cc/bridge.cc", "cc/process.cc", "include/process.h"];
    watched.iter().for_each(|file| println!("cargo:rerun-if-changed=src/{}", file));
}
