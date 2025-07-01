pub mod config;
pub mod file;
pub mod helpers;
pub mod log;
pub mod process;



#[cxx::bridge]
pub mod service {
    #[repr(u8)]
    enum Fork {
        Parent,
        Child,
    }

    pub struct ProcessMetadata {
        pub name: String,
        pub shell: String,
        pub command: String,
        pub log_path: String,
        pub args: Vec<String>,
        pub env: Vec<String>,
    }

    unsafe extern "C++" {
        include!("pmc/lib/include/process.h");
        include!("pmc/lib/include/bridge.h");


        pub fn stop(pid: i64) -> i64;

        pub fn run(metadata: ProcessMetadata) -> i64;
        pub fn find_chidren(parentPID: i64) -> Vec<i64>;


    }
}

// Re-export Rust implementations outside of cxx bridge
pub use process::get_process_cpu_usage_percentage;
