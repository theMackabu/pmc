fn main() {
    cxx_build::bridge("src/main.rs").file("src/cmd.cc").flag_if_supported("-std=c++14").compile("cmd");

    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=src/cmd.cc");
    println!("cargo:rerun-if-changed=src/include/cmd.h");
}
