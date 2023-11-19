mod fork;
mod log;

use chrono::{DateTime, Utc};
use colored::Colorize;
use fork::{daemon, Fork};
use global_placeholders::global;
use log::Logger;
use macros_rs::{crashln, str, string, ternary, then};
use psutil::process::{MemoryInfo, Process};
use serde::Serialize;
use serde_json::json;
use std::{process, thread::sleep, time::Duration};

use pmc::{
    config, file,
    helpers::{self, ColoredString},
    process::{id::Id, Runner},
};

use tabled::{
    settings::{
        object::Columns,
        style::{BorderColor, Style},
        themes::Colorization,
        Color, Rotate,
    },
    Table, Tabled,
};

extern "C" fn handle_termination_signal(_: libc::c_int) {
    pid::remove();
    let mut log = Logger::new().unwrap();
    log.write(format!("daemon killed (pid={})", process::id()).as_str());
    unsafe { libc::_exit(0) }
}

fn restart_process(runner: Runner) {
    let mut log = Logger::new().unwrap();
    let items = runner.list().iter().filter_map(|(id, item)| Some((id.trim().parse::<usize>().ok()?, item)));
    for (id, item) in items {
        then!(!item.running || pid::running(item.pid as i32), continue);
        let name = &Some(item.name.clone());
        let mut runner_instance = Runner::new();
        runner_instance.restart(id, name, true);
        log.write(format!("restarted {} ({id})", item.name).as_str());
    }
}

pub fn health(format: &String) {
    let mut pid: Option<i32> = None;
    let mut cpu_percent: Option<f32> = None;
    let mut uptime: Option<DateTime<Utc>> = None;
    let mut memory_usage: Option<MemoryInfo> = None;
    let runner: Runner = file::read(global!("pmc.dump"));

    #[derive(Clone, Debug, Tabled)]
    struct Info {
        #[tabled(rename = "pid file")]
        pid_file: String,
        #[tabled(rename = "fork path")]
        path: String,
        #[tabled(rename = "cpu percent")]
        cpu_percent: String,
        #[tabled(rename = "memory usage")]
        memory_usage: String,
        #[tabled(rename = "daemon type")]
        external: String,
        #[tabled(rename = "process count")]
        process_count: usize,
        uptime: String,
        pid: String,
        status: ColoredString,
    }

    impl Serialize for Info {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let trimmed_json = json!({
             "pid_file": &self.pid_file.trim(),
             "path": &self.path.trim(),
             "cpu": &self.cpu_percent.trim(),
             "mem": &self.memory_usage.trim(),
             "process_count": &self.process_count.to_string(),
             "uptime": &self.uptime.trim(),
             "pid": &self.pid.trim(),
             "status": &self.status.0.trim(),
            });

            trimmed_json.serialize(serializer)
        }
    }

    if pid::exists() {
        if let Ok(process_id) = pid::read() {
            if let Ok(mut process) = Process::new(process_id as u32) {
                pid = Some(process_id);
                uptime = Some(pid::uptime().unwrap());
                memory_usage = process.memory_info().ok();
                cpu_percent = process.cpu_percent().ok();
            }
        }
    }

    let cpu_percent = match cpu_percent {
        Some(percent) => format!("{:.2}%", percent),
        None => string!("0%"),
    };

    let memory_usage = match memory_usage {
        Some(usage) => helpers::format_memory(usage.rss()),
        None => string!("0b"),
    };

    let uptime = match uptime {
        Some(uptime) => helpers::format_duration(uptime),
        None => string!("none"),
    };

    let pid = match pid {
        Some(pid) => string!(pid),
        None => string!("n/a"),
    };

    let data = vec![Info {
        pid: pid,
        cpu_percent,
        memory_usage,
        uptime: uptime,
        path: global!("pmc.base"),
        external: global!("pmc.daemon.kind"),
        process_count: runner.list().keys().len(),
        pid_file: format!("{}  ", global!("pmc.pid")),
        status: ColoredString(ternary!(pid::exists(), "online".green().bold(), "stopped".red().bold())),
    }];

    let table = Table::new(data.clone())
        .with(Rotate::Left)
        .with(Style::rounded().remove_horizontals())
        .with(Colorization::exact([Color::FG_CYAN], Columns::first()))
        .with(BorderColor::filled(Color::FG_BRIGHT_BLACK))
        .to_string();

    if let Ok(json) = serde_json::to_string(&data[0]) {
        match format.as_str() {
            "raw" => println!("{:?}", data[0]),
            "json" => println!("{json}"),
            "default" => {
                println!("{}\n{table}\n", format!("PMC daemon information").on_bright_white().black());
                println!(" {}", format!("Use `pmc daemon restart` to restart the daemon").white());
                println!(" {}", format!("Use `pmc daemon reset` to clean process id values").white());
            }
            _ => {}
        };
    };
}

pub fn stop() {
    if pid::exists() {
        let mut log = Logger::new().unwrap();
        println!("{} Stopping PMC daemon", *helpers::SUCCESS);

        match pid::read() {
            Ok(pid) => {
                pmc::service::stop(pid as i64);
                pid::remove();
                log.write(format!("daemon stopped (pid={pid})").as_str());
                println!("{} PMC daemon stopped", *helpers::SUCCESS);
            }
            Err(err) => crashln!("{} Failed to read PID file: {}", *helpers::FAIL, err),
        }
    } else {
        crashln!("{} The daemon is not running", *helpers::FAIL)
    }
}

pub fn start() {
    let external = match global!("pmc.daemon.kind").as_str() {
        "external" => true,
        "default" => false,
        "rust" => false,
        "cc" => true,
        _ => false,
    };

    pid::name("PMC Restart Handler Daemon");
    println!("{} Spawning PMC daemon (pmc_base={})", *helpers::SUCCESS, global!("pmc.base"));

    if pid::exists() {
        match pid::read() {
            Ok(pid) => then!(!pid::running(pid), pid::remove()),
            Err(_) => crashln!("{} The daemon is already running", *helpers::FAIL),
        }
    }

    extern "C" fn init() {
        let config = config::read();
        let mut log = Logger::new().unwrap();

        unsafe { libc::signal(libc::SIGTERM, handle_termination_signal as usize) };
        pid::write(process::id());
        log.write(format!("new daemon forked (pid={})", process::id()).as_str());

        loop {
            let runner: Runner = file::read(global!("pmc.dump"));
            then!(!runner.list().is_empty(), restart_process(runner));
            sleep(Duration::from_millis(config.daemon.interval));
        }
    }

    println!("{} PMC Successfully daemonized (type={})", *helpers::SUCCESS, global!("pmc.daemon.kind"));
    if external {
        let callback = pmc::Callback(init);
        pmc::service::try_fork(false, false, callback);
    } else {
        match daemon(false, false) {
            Ok(Fork::Parent(_)) => {}
            Ok(Fork::Child) => init(),
            Err(err) => crashln!("{} Daemon creation failed with code {err}", *helpers::FAIL),
        }
    }
}

pub fn restart() {
    if pid::exists() {
        stop();
    }
    start();
}

pub fn reset() {
    let mut runner = Runner::new();
    let largest = runner.list().keys().last().cloned();

    match largest {
        Some(id) => runner.set_id(Id::from(str!(id))),
        None => println!("{} Cannot reset index, no ID found", *helpers::FAIL),
    }

    println!("{} PMC Successfully reset (index={})", *helpers::SUCCESS, runner.id);
}

pub mod pid;
