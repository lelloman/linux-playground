use anyhow::{bail, Context, Result};
use byte_unit::Byte;
use clap::Parser;
use libc::{free, malloc};
use std::collections::VecDeque;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread::{sleep, spawn, JoinHandle};
use std::time::{Duration, Instant};
use rand::Rng;

fn u8_percent(s: &str) -> Result<u8> {
    let parsed = s.parse::<u8>()?;
    if parsed > 100 {
        bail!("Percent value must be at most 100.");
    }
    Ok(parsed)
}

#[derive(Parser, Clone)]
struct CliArgs {
    #[clap(short = 'j', long, default_value_t = 1)]
    threads: u16,

    #[clap(short, long)]
    bytes: Option<u128>,

    #[clap(short, long, default_value_t = 1000)]
    refresh_rate_ms: u16,

    #[clap(long, default_value_t = 200)]
    base_hold_time_ms: u64,

    #[clap(long, default_value_t = 100)]
    stride: usize,

    #[clap(short, long)]
    timeout_seconds: Option<u64>,

    #[clap(long, default_value_t = 1.0)]
    staggered_hold_time_factor: f64,

    #[clap(long)]
    target: Option<u128>,

    #[clap(long, default_value_t = 0, value_parser=u8_percent)]
    rand_data_percent: u8,
}

#[derive(Default)]
struct FreeStats {
    mem_total: u128,
    mem_available: u128,
    swap_total: u128,
    swap_available: u128,
}

#[derive(Default)]
struct ZswapStats {
    written_back: u128,
    rejects: u128,
    pool_size: u128,
}

#[derive(Default)]
struct MemStats {
    free: FreeStats,
    zswap: ZswapStats,
}

struct State {
    target: Byte,
    start_time: Instant,
    mem_stats: MemStats,
    workers: Vec<WorkerState>,
    verifications: u128,
}

enum WorkerState {
    Allocating,
    Holding,
    Verifying,
}

enum Message {
    WorkerState(u16, WorkerState),
    MemStats(MemStats),
    ThreadError(String, String),
    VerificationCompleted,
}

struct ThreadPayload {
    id: String,
    args: CliArgs,
    thread_allocation_size: usize,
    running: Arc<AtomicBool>,
    tx: Sender<Message>,
    rand_data_len: usize,
}

impl ThreadPayload {
    fn clone<T: AsRef<str> + Clone>(&self, id: T) -> ThreadPayload {
        ThreadPayload {
            id: id.as_ref().to_owned(),
            args: self.args.clone(),
            thread_allocation_size: self.thread_allocation_size,
            running: self.running.clone(),
            tx: self.tx.clone(),
            rand_data_len: self.rand_data_len.clone(),
        }
    }

    fn send(&self, msg: Message) {
        self.tx
            .send(msg)
            .expect("Could not send message from thread.");
    }

    fn error(&self, msg: String) {
        self.send(Message::ThreadError(self.id.clone(), msg));
    }
}

fn fmtb(bytes: u128) -> String {
    byte_unit::Byte::from(bytes)
        .get_appropriate_unit(true)
        .to_string()
}

fn exec<T: AsRef<std::ffi::OsStr>>(parts: &[T]) -> Result<String> {
    let mut cmd = Command::new(&parts[0]);
    for i in 1..parts.len() {
        cmd.arg(&parts[i]);
    }

    let output = cmd.output()?;
    if !output.status.success() {
        bail!("Command failed.");
    }
    let s = String::from_utf8_lossy(&output.stdout).into_owned();
    Ok(s)
}

fn parse_free() -> Result<FreeStats> {
    let txt = exec(&["free", "-b"])?;
    let lines: Vec<Vec<String>> = txt
        .lines()
        .map(|x| x.split_whitespace().map(|y| y.to_owned()).collect())
        .collect();
    let mem_line = lines
        .get(1)
        .context("Could not find mem line in free output.")?;
    let mem_total = mem_line
        .get(1)
        .map(|x| x.parse::<u128>())
        .context("Could not find mem total in free output")??;
    let mem_available = mem_line
        .last()
        .map(|x| x.parse::<u128>())
        .context("Could not find mem total in free output")??;
    let mut stats = FreeStats {
        mem_total,
        mem_available,
        ..Default::default()
    };
    if let Some(line) = lines.get(2) {
        if let Some(x) = line.get(0) {
            if x == "Swap:" {
                if let Some(Ok(n)) = line.get(1).map(|x| x.parse::<u128>()) {
                    stats.swap_total = n;
                }
                if let Some(Ok(n)) = line.get(3).map(|x| x.parse::<u128>()) {
                    stats.swap_available = n;
                }
            }
        }
    }
    Ok(stats)
}

fn parse_zswap() -> Result<ZswapStats> {
    Ok(ZswapStats {
        written_back: read_swap_param("written_back_pages")?,
        rejects: read_swap_param("reject_reclaim_fail")?,
        pool_size: read_swap_param("stored_pages")?,
    })
}

fn setup_ctrl(running: Arc<AtomicBool>) {
    ctrlc::set_handler(move || {
        println!("Ctrl-C received.");
        running.store(false, Ordering::SeqCst);
    })
    .expect("Could not set Ctrl-C handler.");
}

fn make_allocation(size: usize, stride: usize, random_data_len: usize) -> *mut libc::c_void {
    unsafe {
        let ptr = malloc(size);
        let slice: &mut [u8] = std::slice::from_raw_parts_mut(ptr as *mut u8, size);
        let mut rng = rand::thread_rng();
        let mut i = 0;
        let mut index = 0;
        if random_data_len > 0 {
            while i < size {
                rng.fill(&mut slice[i..i+random_data_len]);
                i += 4096;
            }
        }

        i = 0;
        while i < size-8 {
            slice[i] = 0x11;
            slice[i + 1] = 0x22;
            slice[i + 2] = 0x33;
            slice[i + 3] = 0x44;
            slice[i + 4] = ((index >> 24) & 0xff) as u8;
            slice[i + 5] = ((index >> 16) & 0xff) as u8;
            slice[i + 6] = ((index >> 8) & 0xff) as u8;
            slice[i + 7] = (index & 0xff) as u8;
            i += stride;
            index += 1;
        }
        ptr
    }
}

fn verify_and_free(size: usize, stride: usize, ptr: *mut libc::c_void) -> Result<()> {
    unsafe {
        let slice = std::slice::from_raw_parts_mut(ptr as *mut u8, size);
        let mut i = 0;
        let mut index = 0;
        let mut ring: VecDeque<[u8; 8]> = VecDeque::new();
        for _ in 0..4 {
            ring.push_back([0u8; 8]);
        }
        while i < size-8 {
            let failed = slice[i] != 0x11
                || slice[i + 1] != 0x22
                || slice[i + 2] != 0x33
                || slice[i + 3] != 0x44
                || slice[i + 4] != ((index >> 24) & 0xff) as u8
                || slice[i + 5] != ((index >> 16) & 0xff) as u8
                || slice[i + 6] != ((index >> 8) & 0xff) as u8
                || slice[i + 7] != (index & 0xff) as u8;
            let mut popped = ring.pop_front().unwrap();
            popped.clone_from_slice(&slice[i..i + 8]);
            ring.push_back(popped);
            if failed {
                let mut msg = String::new();
                msg += &format!("Possible memory corruption at {:p} ({:#x}).", &slice[i], i);
                while !ring.is_empty() {
                    let popped = ring.pop_front().unwrap();
                    msg += &format!("\n{:x?}", popped);
                }
                msg += " <--- THE BAD GUY\n";
                bail!(msg);
            }
            i += stride;
            index += 1;
        }
        free(ptr)
    };
    Ok(())
}

fn spawn_memory_worker(id: u16, payload: ThreadPayload) -> JoinHandle<String> {
    let sleep_time_ms = match payload.args.base_hold_time_ms {
        0 => 0,
        _ =>  {
            if payload.args.staggered_hold_time_factor == 1.0 {
                payload.args.base_hold_time_ms
            } else {
                let f = payload.args.staggered_hold_time_factor as f64;
                let i = id as f64 + 1.0;
                (payload.args.base_hold_time_ms as f64 / i.powf(f)) as u64
            }
        },
    };
    let sleep_duration = Duration::from_millis(sleep_time_ms);

    spawn(move || {
        while payload.running.load(Ordering::SeqCst) {
            payload.send(Message::WorkerState(id, WorkerState::Allocating));
            let ptr = make_allocation(payload.thread_allocation_size, payload.args.stride, payload.rand_data_len);
            if ptr.is_null() {
                payload.error("Allocation failed".to_owned());
                break;
            }
            payload.send(Message::WorkerState(id, WorkerState::Holding));
            sleep(sleep_duration);

            payload.send(Message::WorkerState(id, WorkerState::Verifying));
            if let Err(err) =
                verify_and_free(payload.thread_allocation_size, payload.args.stride, ptr)
            {
                payload.error(format!("Verification error.\n{}", err));
                break;
            } else {
                payload.send(Message::VerificationCompleted);
            }
        }
        payload.id
    })
}

fn read_swap_param(name: &str) -> Result<u128> {
    let txt = std::fs::read_to_string(format!("/sys/kernel/debug/zswap/{}", name))?;
    Ok(txt.trim().parse::<u128>()?)
}

fn spawn_stats_parser(payload: ThreadPayload) -> JoinHandle<String> {
    let sleep_duration = Duration::from_millis(payload.args.refresh_rate_ms.into());
    spawn(move || {
        let send = |payload: &ThreadPayload, msg: Message| {
            payload
                .tx
                .send(msg)
                .expect("Could not send screen renderer message.");
        };
        while payload.running.load(Ordering::SeqCst) {
            let free_stats = match parse_free() {
                Ok(x) => x,
                Err(err) => {
                    payload.error(format!("Error while reading free.\n{}", err));
                    break;
                }
            };
            let zswap_stats = match parse_zswap() {
                Ok(x) => x,
                Err(err) => {
                    payload.error(format!("Error while reading zswap stats.\n{}", err));
                    break;
                }
            };
            let stats = MemStats {
                free: free_stats,
                zswap: zswap_stats,
                ..Default::default()
            };
            send(&payload, Message::MemStats(stats));
            sleep(sleep_duration);
        }
        payload.id
    })
}

fn fmt_duration(seconds: u64) -> String {
    let minutes = (seconds / 60) % 60;
    let hours = (seconds / 60) / 60;
    let seconds = seconds % 60;
    format!("{:0>2}:{:0>2}:{:0>2}", hours, minutes, seconds)
}

fn get_available_memory() -> Result<u128> {
    let free_stats =
        parse_free().context("Could not determine target allocation, failed to parse free.")?;
    let swap_percent = (free_stats.swap_available as f64 * 0.4) as u128;
    Ok(free_stats.mem_available + swap_percent)
}

fn compute_thread_allocation_size(args: &CliArgs) -> Result<usize> {
    let tot_bytes = match args.bytes {
        Some(x) => x,
        None => get_available_memory()?,
    };
    let threads: u128 = args.threads.into();
    let per_thread: usize = (tot_bytes / threads) as usize;
    let out = per_thread + (4096 - (per_thread % 4096));
    assert!(out % 4096 == 0);
    Ok(out)
}

fn print_row(items: &[&str], alignments: &str) {
    assert!(items.len() == alignments.len());
    let n_cells = items.len();
    items.into_iter().enumerate().for_each(|(i, item)| {
        let align_char = alignments.chars().nth(i).unwrap();
        match (n_cells, align_char) {
            (1, '<') => print!("{: <60}", item),
            (1, '>') => print!("{: >60}", item),
            (2, '<') => print!("{: <30}", item),
            (2, '>') => print!("{: >30}", item),
            (3, '<') => print!("{: <20}", item),
            (3, '>') => print!("{: >20}", item),
            (3, '^') => print!("{: ^20}", item),
            _ => panic!(
                "Invalid row values len {} align_char {}.",
                n_cells, align_char
            ),
        };
    });
    print!("\n");
}

fn render_free_stats(stats: &FreeStats) {
    print_row(&["MEMORY", "available", "total"], "<>>");
    print_row(
        &["Mem", &fmtb(stats.mem_available), &fmtb(stats.mem_total)],
        "<>>",
    );
    print_row(
        &["Swap", &fmtb(stats.swap_available), &fmtb(stats.swap_total)],
        "<>>",
    );
}

fn render_zswap_stats(stats: &ZswapStats) {
    print_row(&["ZSWAP", "bytes", "pages"], "<>>");
    print_row(
        &[
            "size",
            &fmtb(stats.pool_size*4096),
            &stats.pool_size.to_string(),
        ],
        "<>>",
    );
    print_row(
        &[
            "writebacks",
            &fmtb(stats.written_back * 4096),
            &stats.written_back.to_string(),
        ],
        "<>>",
    );
    print_row(&["failures", "-", &stats.rejects.to_string()], "<>>");
}

const ALLOCATING_VEC: [&str; 3] = ["X", "", ""];
const HOLDING_VEC: [&str; 3] = ["", "X", ""];
const VERIFYING_VEC: [&str; 3] = ["", "", "X"];

fn render_workers_states(states: &[WorkerState]) {
    print_row(&["Allocating", "Holding", "Verifying"], "^^^");
    states.iter().for_each(|state| {
        let v = match state {
            WorkerState::Allocating => ALLOCATING_VEC,
            WorkerState::Holding => HOLDING_VEC,
            WorkerState::Verifying => VERIFYING_VEC,
        };
        print_row(&v, "^^^");
    });
}

fn render_state(state: &State) {
    print!("{}[2J", 27 as char);
    print!("{esc}[2J{esc}[1;1H", esc = 27 as char);

    let duration_str = fmt_duration(state.start_time.elapsed().as_secs());
    print_row(&["Elapsed:", &duration_str], "<>");

    let target_str = state.target.get_appropriate_unit(true).to_string();
    print_row(&["Target:", &target_str], "<>");

    print_row(&["Verifications:", &state.verifications.to_string()], "<>");

    println!("");
    render_free_stats(&state.mem_stats.free);
    println!("");
    render_zswap_stats(&state.mem_stats.zswap);
    println!("");
    render_workers_states(&state.workers);
}

fn main() {
    let args = CliArgs::parse();

    let thread_allocation_size =
        compute_thread_allocation_size(&args).expect("Could not determine allocation size.");
    let running = Arc::new(AtomicBool::new(true));
    let mut state = State {
        target: Byte::from_bytes((thread_allocation_size as u128) * args.threads as u128),
        start_time: Instant::now(),
        mem_stats: MemStats::default(),
        workers: (0..args.threads).map(|_| WorkerState::Allocating).collect(),
        verifications: 0,
    };

    setup_ctrl(running.clone());
    let (tx, rx): (Sender<Message>, Receiver<Message>) = channel();

    let rand_data_len: usize = (args.rand_data_percent as usize * 4096) / 100;
    let payload = ThreadPayload {
        id: "".to_owned(),
        args: args.clone(),
        thread_allocation_size: thread_allocation_size,
        running: running.clone(),
        tx: tx.clone(),
        rand_data_len: rand_data_len,
    };

    let mut join_handles: Vec<JoinHandle<String>> = Vec::new();

    join_handles.push(spawn_stats_parser(payload.clone("stats")));

    for i in 0..args.threads {
        join_handles.push(spawn_memory_worker(
            i,
            payload.clone(format!("worker-{}", i)),
        ))
    }

    let rcv_timeout = Duration::from_secs(1);
    let start_time = Instant::now();
    let timeout_secs = args.timeout_seconds.unwrap_or(u64::MAX);
    while running.load(Ordering::SeqCst) {
        match rx.recv_timeout(rcv_timeout) {
            Ok(Message::MemStats(stats)) => {
                state.mem_stats = stats;
                render_state(&state);
            }
            Ok(Message::WorkerState(worker_id, worker_state)) => {
                state.workers[worker_id as usize] = worker_state;
                render_state(&state);
            }
            Ok(Message::ThreadError(id, txt)) => {
                //running.store(false, Ordering::SeqCst);
                println!("Thread <{}> sent an error.\n{}", id, txt);
                Command::new("cat")
                    .arg("/sys/kernel/debug/tracing/trace")
                    .stdout(std::fs::File::create("/shared/logs.txt").unwrap())
                    .spawn()
                    .expect("Failed to cat to logs.txt");
                break;
            }
            Ok(Message::VerificationCompleted) => {
                state.verifications += 1;
                render_state(&state);
                if let Some(target) = args.target {
                    if target == state.verifications {
                        running.store(false, Ordering::SeqCst);
                        break;
                    }
                }
            }
            Err(_) => {}
        }
        if start_time.elapsed().as_secs() >= timeout_secs {
            running.store(false, Ordering::SeqCst);
        }
    }

    while running.load(Ordering::SeqCst) {
        sleep(Duration::from_millis(500));
    }
    println!("Shutting down, waiting for threads to join...");
    while !join_handles.is_empty() {
        let joined = join_handles
            .pop()
            .unwrap()
            .join()
            .expect("Could not join thread");
        println!("{} joined.", joined);
    }
    println!("Done.");
}
