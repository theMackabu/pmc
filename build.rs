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

    println!("cargo:rustc-env=NODE_HOME={}", node_extract_dir.to_str().unwrap());

    return node_extract_dir;
}

fn download_then_build(node_extract_dir: PathBuf) {
    let base_dir = match fs::canonicalize(node_extract_dir) {
        Ok(path) => path,
        Err(err) => panic!("{err}"),
    };

    let bin = &base_dir.join("bin");
    let node = &bin.join("node");
    let project_dir = &Path::new("src").join("webui");
    let npm = &base_dir.join("lib/node_modules/npm/index.js");

    /* set path */
    let mut paths = match env::var_os("PATH") {
        Some(paths) => env::split_paths(&paths).collect::<Vec<PathBuf>>(),
        None => vec![],
    };

    paths.push(bin.clone());

    let path = match env::join_paths(paths) {
        Ok(joined) => joined,
        Err(err) => panic!("{err}"),
    };

    /* install deps */
    Command::new(node)
        .args([npm.to_str().unwrap(), "ci"])
        .current_dir(project_dir)
        .env("PATH", &path)
        .status()
        .expect("Failed to install dependencies");

    /* build frontend */
    Command::new(node)
        .args(["node_modules/astro/astro.js", "build"])
        .current_dir(project_dir)
        .env("PATH", &path)
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

            #[allow(unused_must_use)]
            for name in vec!["assets", "dist"] {
                fs::remove_dir_all(format!("src/webui/{name}"));
            }

            /* pre-build */
            let path = download_node();
            download_then_build(path);

            /* move assets */
            fs::create_dir_all("src/webui/assets/").expect("Failed to move assets");
            fs::rename("src/webui/dist/static", "src/webui/assets/static").expect("Failed to move assets");

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

    let watched = vec![
        "lib",
        "src/lib.rs",
        "lib/include",
        "src/webui/src",
        "src/webui/links.ts",
        "src/webui/package.json",
        "src/webui/tsconfig.json",
        "src/webui/astro.config.mjs",
        "src/webui/tailwind.config.mjs",
    ];

    watched.iter().for_each(|file| println!("cargo:rerun-if-changed={file}"));
}
