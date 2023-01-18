use clap::Parser;
use lazy_static::lazy_static;
use rand::prelude::*;
use rand::seq::SliceRandom;
use regex::Regex;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use libc::{c_void, syscall};

const MIN_ALLOCATION_SIZE: u64 = 1 << 12;
const MAX_ALLOCATION_SIZE: u64 = 1 << 20;
const START_ALLOCATION_SIZE: u64 = 1 << 12;
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

fn kmalloc(size: u64) -> Option<*mut c_void> {
    let ret = unsafe { syscall(548, size) as *mut c_void };
    let v = ret as usize;
    if v == 0 || v == 0xffffffff {
        None
    } else {
        Some(ret)
    }
}

fn kfree(address: *const c_void) -> *const c_void {
    unsafe { syscall(549, address) as *const c_void }
}

fn kmeminfo() {
    unsafe { syscall(550) };
}

struct Allocation {
    pub ptr: *const c_void,
    pub bytes_count: u64,
}

fn free_all(allocations: &mut Vec<Allocation>, print: bool) {
    let mut tot_allocated = 0u64;
    while allocations.len() > 0 {
        let a = allocations.pop().unwrap();
        tot_allocated += a.bytes_count;
        kfree(a.ptr);
    }
    if print {
        println!("freed {}", fmt_bytes(tot_allocated));
    }
}

enum AllocationAction {
    FREE(u64),
    ALLOC(bool, u64),
    NONE,
}

impl fmt::Display for AllocationAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AllocationAction::FREE(size) => write!(f, "F({})    ", fmt_bytes(*size)),
            AllocationAction::ALLOC(success, size) => {
                let success_str = if *success { "OK" } else { "NO" };
                write!(f, "A({}) {}", fmt_bytes(*size), success_str)
            }
            AllocationAction::NONE => write!(f, "NONE"),
        }
    }
}

fn pick_target_allocation(
    allocations_map: &mut HashMap<u64, Vec<Allocation>>,
) -> Option<Allocation> {
    let non_empty_keys: Vec<u64> = allocations_map
        .iter()
        .filter(|(_, v)| v.len() > 0)
        .map(|(k, _)| *k)
        .collect();
    if non_empty_keys.is_empty() {
        return None;
    }
    let key = non_empty_keys
        .choose(&mut rand::thread_rng())
        .expect("We checked that it was not empty...?");

    let allocations = allocations_map
        .get_mut(key)
        .expect("There must be a value for this key, we just checked it was not empty!");

    let (allocation_index, _) = allocations
        .iter()
        .enumerate()
        .choose(&mut rand::thread_rng())
        .expect("then why we called it 'non_empty_keys'...???");
    Some(allocations.remove(allocation_index))
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
    allocations: HashMap<u64, Vec<Allocation>>,
    should_allocate_next: bool,
    current_allocation_size: u64,
    abs_peak: u64,
    latest_results: VecDeque<AllocationAction>,
    tot_allocated: u64,
    max_allocation: u64,
    alloc_failures: u64,
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
            max_allocation,
            allocations: HashMap::new(),
            should_allocate_next: true,
            current_allocation_size: START_ALLOCATION_SIZE,
            abs_peak: 0u64,
            latest_results: VecDeque::from([AllocationAction::NONE]),
            tot_allocated: 0,
            alloc_failures: 0,
        }
    }

    fn do_allocation_step(&mut self) {
        match kmalloc(self.current_allocation_size) {
            Some(x) => {
                self.allocations
                    .entry(self.current_allocation_size)
                    .or_default()
                    .push(Allocation {
                        ptr: x,
                        bytes_count: self.current_allocation_size,
                    });
                self.latest_results
                    .push_front(AllocationAction::ALLOC(true, self.current_allocation_size));
                self.tot_allocated += self.current_allocation_size;
                self.abs_peak = std::cmp::max(self.tot_allocated, self.abs_peak);
            }
            None => {
                self.alloc_failures += 1;
                self.latest_results
                    .push_front(AllocationAction::ALLOC(false, self.current_allocation_size));
                if self.current_allocation_size <= MIN_ALLOCATION_SIZE {
                    self.should_allocate_next = false;
                } else {
                    self.current_allocation_size = self.current_allocation_size >> 1;
                }
            }
        }
    }

    fn do_free_step(&mut self) {
        match pick_target_allocation(&mut self.allocations) {
            None => {
                self.current_allocation_size = START_ALLOCATION_SIZE;
                self.should_allocate_next = true;
                self.latest_results.push_front(AllocationAction::NONE);
            }
            Some(allocation) => {
                kfree(allocation.ptr);
                self.tot_allocated -= allocation.bytes_count;
                self.latest_results
                    .push_front(AllocationAction::FREE(allocation.bytes_count));
            }
        }
    }

    fn are_latest_results_all_alloc_failure(&self) -> bool {
        self.latest_results.iter().all(|x| match x {
            AllocationAction::ALLOC(success, _) => !success,
            _ => false,
        })
    }

    fn are_latest_results_all_alloc_success(&self) -> bool {
        self.latest_results.iter().all(|x| {
            if let AllocationAction::ALLOC(true, _) = x {
                true
            } else {
                false
            }
        })
    }

    fn are_latest_results_all_free(&self) -> bool {
        self.latest_results.iter().all(|x| {
            if let AllocationAction::FREE(_) = x {
                true
            } else {
                false
            }
        })
    }

    fn adjust_according_to_latest_results(&mut self) {
        if self.latest_results.len() >= 3 {
            if self.tot_allocated >= self.max_allocation
                || self.are_latest_results_all_alloc_failure()
            {
                self.should_allocate_next = false;
            } else if self.are_latest_results_all_free() && self.tot_allocated < self.abs_peak / 4 {
                self.should_allocate_next = true;
            } else if self.are_latest_results_all_alloc_success() {
                self.current_allocation_size =
                    std::cmp::min(MAX_ALLOCATION_SIZE, self.current_allocation_size << 1);
            }
        }
    }

    fn print_step_report(&self) {
        let last_action = self.latest_results.front().unwrap();
        println!(
            "{: >8} {: >15} - peak {}",
            fmt_bytes(self.tot_allocated),
            last_action,
            fmt_bytes(self.abs_peak),
        );
    }

    fn run(&mut self) {
        let start = std::time::Instant::now();
        while self.running.load(Ordering::SeqCst) && start.elapsed() < self.duration {
            if self.should_allocate_next {
                self.do_allocation_step();
            } else {
                self.do_free_step();
            }
            self.latest_results.truncate(3);
            self.adjust_according_to_latest_results();
            if self.verbose {
                self.print_step_report();
            }
        }
        //kmeminfo();
        self.allocations.values_mut().for_each(|allocations| {
            free_all(allocations, self.verbose);
        });
        //kmeminfo();
    }
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

    let duration = Duration::from_millis(args.duration_ms);
    let interval = Duration::from_millis(args.interval_ms);
    let mut peaks = Vec::<u64>::new();
    let mut iteration = 0;
    while iteration < args.iterations && running.load(Ordering::SeqCst) {
        let mut program =
            AllocatorProgram::new(running.clone(), duration, args.verbose, args.max_allocation);
        program.run();
        peaks.push(program.abs_peak);
        iteration += 1;
        println!("Iter {} peaked {} alloc failures {}", iteration, fmt_bytes(program.abs_peak), program.alloc_failures);
        std::thread::sleep(interval);
    }

    let max_peak = *peaks.iter().max().unwrap_or(&0);
    let min_peak = *peaks.iter().min().unwrap_or(&0);
    //kmeminfo();
    println!("Run {} iterations", iteration);
    println!(
        "Min/max peaks: {} {}",
        fmt_bytes(min_peak),
        fmt_bytes(max_peak)
    );
}
