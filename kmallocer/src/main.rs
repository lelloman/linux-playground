use clap::Parser;
use lazy_static::lazy_static;
use rand::prelude::*;
use rand::seq::SliceRandom;
use regex::Regex;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

const DEFAULT_DURATION_MS: u64 = 100;
const DEFAULT_ITERATIONS: u16 = 3;
const DEFAULT_ITERATIONS_INTERVAL_MS: u64 = 100;
const DEFAULT_MAX_ALLOCATION: u64 = 50_000_000;

lazy_static! {
    static ref NUMBER_REGEX: Regex = Regex::new(r"^\d+$").unwrap();
}

#[derive(Parser, Debug)]
struct CliArgs {
    #[clap(short, long, default_value_t = DEFAULT_DURATION_MS)]
    duration_ms: u64,

    #[clap(short, long, default_value_t = DEFAULT_ITERATIONS)]
    iterations: u16,

    #[clap(short='s', long, default_value_t = DEFAULT_ITERATIONS_INTERVAL_MS)]
    interval_ms: u64,

    #[clap(short, long)]
    verbose: bool,

    #[clap(short, long, default_value_t = DEFAULT_MAX_ALLOCATION)]
    max_allocation: u64,
}

fn fmt_bytes(bytes: u64) -> String {
    let (suffix, divisor) = match bytes {
        0..=999 => (" ", 1.0),
        1000..=999_999 => ("K", 1_000.0),
        _ => ("M", 1_000_000.0),
    };
    let v = (bytes as f64) / divisor;
    format!("{: >8.1}{}", v, suffix)
}

struct AllocatorProgram {
    running: Arc<AtomicBool>,
    duration: Duration,
    verbose: bool,
}

impl AllocatorProgram {
    fn new(
        running: Arc<AtomicBool>,
        duration: Duration,
        verbose: bool,
        max_allocation: u64,
    ) -> AllocatorProgram {
        AllocatorProgram {
            running,
            duration,
            verbose,
        }
    }

    fn run(&mut self) {
        
    }
}

fn run_kmalloc() -> u64 {
    let mut buffer = [0u8;1024];
    let mut file = File::open("/proc/kmallocer").expect("Could not open proc kmallocer");

    let read_count = file.read(&mut buffer).expect("Could not read from proc kmallocer file");

    let output = String::from_utf8_lossy(&buffer[..read_count]);
    let values = output.split(" ").collect::<Vec<&str>>();
    
    return values[0].parse::<u64>().expect("Could not parse /proc/kmallocer output");
}

fn main() {
    let args = CliArgs::parse();

    let running = Arc::new(AtomicBool::new(true));
    let running_c = running.clone();
    ctrlc::set_handler(move || {
        println!("");
        running_c.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let interval = Duration::from_millis(args.interval_ms);
    let mut peaks = Vec::<u64>::new();
    let mut iteration = 0;

    while iteration < args.iterations && running.load(Ordering::SeqCst) {
        iteration += 1;
        let allocated = run_kmalloc();        
        peaks.push(allocated);
        println!("Iter {} peaked {}", iteration, fmt_bytes(allocated));
        std::thread::sleep(interval);
    }

    let max_peak = *peaks.iter().max().unwrap_or(&0);
    let min_peak = *peaks.iter().min().unwrap_or(&0);
    println!("Run {} iterations", iteration);
    println!(
        "Min/max peaks: {} {}",
        fmt_bytes(min_peak),
        fmt_bytes(max_peak)
    );
}
