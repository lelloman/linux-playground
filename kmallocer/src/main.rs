use lazy_static::lazy_static;
use rand::prelude::*;
use rand::seq::SliceRandom;
use regex::Regex;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use libc::{c_void, syscall};

lazy_static! {
    static ref NUMBER_REGEX: Regex = Regex::new(r"^\d+$").unwrap();
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

struct Allocation {
    pub ptr: *const c_void,
    pub bytes_count: u64,
}

const MIN_ALLOCATION_SIZE: u64 = 1 << 10;
const MAX_ALLOCATION_SIZE: u64 = 1 << 24;

fn free_all(allocations: &mut Vec<Allocation>) {
    let mut tot_allocated = 0u64;
    while allocations.len() > 0 {
        let a = allocations.pop().unwrap();
        tot_allocated += a.bytes_count;
        kfree(a.ptr);
    }
    println!("freed {} bytes", tot_allocated);
}

fn tot_bytes(allocations: &HashMap<u64, Vec<Allocation>>) -> u64 {
    allocations
        .iter()
        .fold(0u64, |a, b| a + b.0 * b.1.len() as u64)
}
#[derive(Debug)]
enum AllocationAction {
    FREE(u64),
    ALLOC(bool, u64),
    NONE,
}

impl fmt::Display for AllocationAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AllocationAction::FREE(size) => write!(f, "FREE({})", fmt_bytes(*size)),
            AllocationAction::ALLOC(success, size) => {
                let success_str = if *success { "OK" } else { "FAILED" };
                write!(f, "ALLOC({}) {}", fmt_bytes(*size), success_str)
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
        0..=999 => ("", 1.0),
        1000..=999_999 => ("Kb", 1_000.0),
        _ => ("M", 1_000_000.0),
    };
    let v = (bytes as f64) / divisor;
    format!("{:.2}{}", v, suffix)
}

fn main() {
    let mut allocations: HashMap<u64, Vec<Allocation>> = HashMap::new();

    let running = Arc::new(AtomicBool::new(true));
    let running_c = running.clone();
    ctrlc::set_handler(move || {
        running_c.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let sleep_time = Duration::from_millis(50);
    let mut should_allocate_next = true;
    let start_allocation_size = 1 << 12;
    let mut current_allocation_size = start_allocation_size;
    let mut abs_peak = 0u64;
    let mut local_peak = 0u64;
    let mut latest_results: VecDeque<AllocationAction> = VecDeque::from([AllocationAction::NONE]);

    let mut tot_allocated = tot_bytes(&allocations);

    while running.load(Ordering::SeqCst) {
        if should_allocate_next {
            match kmalloc(current_allocation_size) {
                Some(x) => {
                    allocations
                        .entry(current_allocation_size)
                        .or_default()
                        .push(Allocation {
                            ptr: x,
                            bytes_count: current_allocation_size,
                        });
                    latest_results
                        .push_front(AllocationAction::ALLOC(true, current_allocation_size));
                    tot_allocated += current_allocation_size;
                    abs_peak = std::cmp::max(tot_allocated, abs_peak);
                    local_peak = std::cmp::max(tot_allocated, local_peak);
                }
                None => {
                    latest_results
                        .push_front(AllocationAction::ALLOC(false, current_allocation_size));
                    if current_allocation_size <= MIN_ALLOCATION_SIZE {
                        should_allocate_next = false;
                        local_peak = 0u64;
                    } else {
                        current_allocation_size = current_allocation_size >> 1;
                    }
                }
            }
        } else {
            match pick_target_allocation(&mut allocations) {
                None => {
                    current_allocation_size = start_allocation_size;
                    should_allocate_next = true;
                    latest_results.push_front(AllocationAction::NONE);
                }
                Some(allocation) => {
                    tot_allocated -= allocation.bytes_count;
                    latest_results.push_front(AllocationAction::FREE(allocation.bytes_count));
                }
            }
        }
        latest_results.truncate(3);

        if latest_results.len() >= 3 {
            if latest_results.iter().all(|x| match x {
                AllocationAction::ALLOC(success, _) => !success,
                _ => false,
            }) {
                should_allocate_next = false;
                local_peak = 0u64;
            } else if latest_results.iter().all(|x| {
                if let AllocationAction::FREE(_) = x {
                    true
                } else {
                    false
                }
            }) && tot_allocated < abs_peak / 4
            {
                should_allocate_next = true;
            } else if latest_results.iter().all(|x| {
                if let AllocationAction::ALLOC(true, _) = x {
                    true
                } else {
                    false
                }
            }) {
                current_allocation_size =
                    std::cmp::min(MAX_ALLOCATION_SIZE, current_allocation_size << 1);
            }
        }

        let last_action = latest_results.front().unwrap();
        println!(
            "{} Current allocation: {} bytes - peaks (abs,loc) {} / {}\n",
            last_action,
            fmt_bytes(tot_allocated),
            fmt_bytes(abs_peak),
            fmt_bytes(local_peak)
        );

        sleep(sleep_time);
    }

    allocations.values_mut().for_each(|allocations| {
        free_all(allocations);
    });
}
