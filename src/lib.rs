pub mod config;
pub mod file;
pub mod helpers;
pub mod log;
pub mod process;

// Deprecated
// #[cxx::bridge]
// pub mod service {
//     #[repr(u8)]
//     enum Fork {
//         Parent,
//         Child,
//     }

//     pub struct ProcessMetadata {
//         pub name: String,
//         pub shell: String,
//         pub command: String,
//         pub log_path: String,
//         pub args: Vec<String>,
//         pub env: Vec<String>,
//     }
// }

// Re-export Rust implementations outside of cxx bridge
pub use process::{
    get_process_cpu_usage_percentage, process_find_children, process_run, process_stop,
};
