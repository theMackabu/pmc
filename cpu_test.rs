use pmc::process::get_process_cpu_usage_percentage;
use std::thread;
use std::time::Duration;

fn main() {
    let test_pid = 16103; // yes 프로세스 PID
    
    println!("Testing CPU usage measurement for PID {}", test_pid);
    println!("Measuring CPU usage over 5 seconds...\n");
    
    for i in 1..=5 {
        let cpu_usage = get_process_cpu_usage_percentage(test_pid);
        println!("Measurement {}: CPU usage = {:.2}%", i, cpu_usage);
        
        if i < 5 {
            thread::sleep(Duration::from_secs(1));
        }
    }
    
    println!("\nTesting with current process PID:");
    let current_pid = std::process::id() as i64;
    let current_cpu = get_process_cpu_usage_percentage(current_pid);
    println!("Current process CPU usage: {:.2}%", current_cpu);
} 