use chrono::Datelike;
use flate2::read::GzDecoder;
use reqwest;
use tar::Archive;

use std::{
    env,
    fs::{self, File},
    io::{self, copy},
    path::{Path, PathBuf},
    process::Command,
};

const NODE_VERSION: &str = "20.10.0";
const PNPM_VERSION: &str = "8.11.0";

fn extract_tar_gz(tar: &PathBuf, download_dir: &PathBuf) -> io::Result<()> {
    let file = File::open(tar)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    archive.unpack(download_dir)?;
    Ok(fs::remove_file(tar)?)
}

fn download_file(url: String, destination: &PathBuf, download_dir: &PathBuf) {
    if !download_dir.exists() {
        fs::create_dir_all(download_dir).unwrap();
    }

    let mut response = reqwest::blocking::get(url).expect("Failed to send request");
    let mut file = File::create(destination).expect("Failed to create file");

    copy(&mut response, &mut file).expect("Failed to copy content");
}

fn download_node() -> PathBuf {
    #[cfg(target_os = "linux")]
    let target_os = "linux";
    #[cfg(all(target_os = "macos"))]
    let target_os = "darwin";

    #[cfg(all(target_arch = "arm"))]
    let target_arch = "armv7l";
    #[cfg(all(target_arch = "x86_64"))]
    let target_arch = "x64";
    #[cfg(all(target_arch = "aarch64"))]
    let target_arch = "arm64";

    let download_url = format!("https://nodejs.org/dist/v{NODE_VERSION}/node-v{NODE_VERSION}-{target_os}-{target_arch}.tar.gz");

    /* paths */
    let download_dir = Path::new("target").join("downloads");
    let node_extract_dir = download_dir.join(format!("node-v{NODE_VERSION}-{target_os}-{target_arch}"));

    if node_extract_dir.is_dir() {
        return node_extract_dir;
    }

    /* download node */
    let node_archive = download_dir.join(format!("node-v{}-{}.tar.gz", NODE_VERSION, target_os));
    download_file(download_url, &node_archive, &download_dir);

    /* extract node */
    if let Err(err) = extract_tar_gz(&node_archive, &download_dir) {
        panic!("Failed to extract Node.js: {:?}", err)
    }

    /* set env */
    println!(
        "cargo:rerun-if-env-changed=NODE_HOME\n\
         cargo:rerun-if-env-changed=PATH\n\
         cargo:rerun-if-env-changed=PNPM_HOME"
    );

    let path = match env::var("PATH") {
        Ok(path) => path,
        Err(err) => panic!("{err}"),
    };

    let mut paths = env::var_os("PATH").unwrap_or_default();

    paths.push(":");
    paths.push(std::ffi::OsString::from(&node_extract_dir));
    env::set_var("PATH", paths);

    println!("cargo:rustc-env=NODE_HOME={}", node_extract_dir.to_str().unwrap());
    println!("cargo:rustc-env=PATH={}/bin:{path}/bin/node", node_extract_dir.to_str().unwrap());

    return node_extract_dir;
}

fn download_then_build(node_extract_dir: PathBuf) {
    /* install pnpm */
    Command::new("./npm")
        .args(["install", "-g", &format!("pnpm@{}", PNPM_VERSION)])
        .current_dir(&node_extract_dir.join("bin"))
        .env("NODE_PATH", &node_extract_dir.join("lib").join("node_modules"))
        .status()
        .expect("Failed to install PNPM");

    /* install deps */
    Command::new("pnpm")
        .args(["install"])
        .current_dir("src/webui")
        .env("NODE_PATH", &node_extract_dir.join("lib").join("node_modules"))
        .status()
        .expect("Failed to install dependecies");

    /* build frontend */
    Command::new(format!("./{}/npx", &node_extract_dir.join("bin").as_path().display()))
        .args(["astro", "build"])
        .current_dir("src/webui")
        .env("NODE_PATH", &node_extract_dir.join("lib").join("node_modules"))
        .status()
        .expect("Failed to build frontend");
}

fn main() {
    #[cfg(target_os = "windows")]
    compile_error!("This project is not supported on Windows.");

    #[cfg(target_arch = "x86")]
    compile_error!("This project is not supported on 32 bit.");

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
        "release" => {
            println!("cargo:rustc-env=PROFILE=release");

            /* pre-build */
            let path = download_node();
            download_then_build(path);

            /* cc linking */
            cxx_build::bridge("src/lib.rs")
                .file("lib/bridge.cc")
                .file("lib/process.cc")
                .file("lib/fork.cc")
                .include("lib/include")
                .flag_if_supported("-std=c++17")
                .compile("bridge");
        }
        _ => println!("cargo:rustc-env=PROFILE=none"),
    }
}
