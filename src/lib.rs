pub mod config;
pub mod file;
pub mod helpers;
pub mod log;
pub mod process;

#[repr(transparent)]
pub struct Callback(pub extern "C" fn());

unsafe impl cxx::ExternType for Callback {
    type Id = cxx::type_id!("Callback");
    type Kind = cxx::kind::Trivial;
}

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
    }

    unsafe extern "C++" {
        include!("pmc/src/include/process.h");
        include!("pmc/src/include/bridge.h");
        include!("pmc/src/include/fork.h");
        type Callback = crate::Callback;

        pub fn stop(pid: i64) -> i64;
        pub fn run(metadata: ProcessMetadata) -> i64;
        pub fn try_fork(nochdir: bool, noclose: bool, callback: Callback) -> i32;
    }
}
