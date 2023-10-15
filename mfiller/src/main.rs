use byte_unit::{Byte, ByteError};
use clap::{ValueEnum, Parser};
use rand::seq::SliceRandom;
use std::fs::{create_dir, remove_dir_all, File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::{fmt, thread};

#[derive(ValueEnum, Clone, Debug)]
enum Mode {
    CatRandom,
    DdZeros,
}

#[derive(Parser, Debug)]
struct CliArgs {
    #[clap(short='s', long, default_value = "1M", value_parser = parse_size_string)]
    file_size: u128,

    #[clap(short = 'c', long, default_value_t = 300)]
    file_count: u16,

    #[clap(short, long)]
    once: bool,

    #[clap(short, long, default_value_t = 1)]
    threads: u16,

    #[clap(short, long, value_enum, default_value_t = Mode::DdZeros)]
    mode: Mode,
}

#[derive(Debug, Clone)]
struct ParseError;
impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Invalid size")
    }
}
impl std::error::Error for ParseError {}

fn parse_size_string(size: &str) -> Result<u128, ByteError> {
    Ok(Byte::from_str(size)?.get_bytes())
}

fn write_content(file_path: &PathBuf, n: u128) {
    let from_file = File::open("/dev/random").expect("Could not open /dev/random");
    let mut reader = BufReader::new(from_file);
    let to_file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(file_path)
        .expect("Could not open file for write");
    let mut writer = BufWriter::new(to_file);
    let mut buf = [0; 1 << 16];

    let mut bytes_copied: u128 = 0;
    while bytes_copied < n {
        let bytes_to_read = std::cmp::min(buf.len() as u128, n - bytes_copied);
        let bytes_read: u128 = reader
            .read(&mut buf[..bytes_to_read as usize])
            .expect("Could not read") as u128;
        if bytes_read == 0 {
            break;
        }
        let bytes_to_write = if bytes_copied + bytes_read as u128 <= n {
            bytes_read
        } else {
            n as u128 - bytes_read
        } as usize;
        writer
            .write_all(&buf[..bytes_to_write])
            .expect("Could not write");
        bytes_copied += bytes_to_write as u128;
    }
    writer.flush().expect("Could not flush writer");
}

fn cat_file(from_file_path: &PathBuf) {
    let to_file = File::create("/dev/null").unwrap();
    let from_file = File::open(from_file_path).expect("Could not open file");
    let n = from_file.metadata().unwrap().len();
    let mut reader = BufReader::new(from_file);
    let mut writer = BufWriter::new(to_file);
    let mut buf = [0; 1 << 16];

    let mut bytes_copied: u64 = 0;
    while bytes_copied < n {
        let bytes_to_read = std::cmp::min(buf.len() as u64, n - bytes_copied);
        let bytes_read = reader
            .read(&mut buf[..bytes_to_read as usize])
            .expect("Could not read");
        if bytes_read == 0 {
            break;
        }
        let bytes_to_write = if bytes_copied + bytes_read as u64 <= n {
            bytes_read
        } else {
            (n - bytes_copied) as usize
        } as usize;
        writer
            .write_all(&buf[..bytes_to_write])
            .expect("Could not write");
        bytes_copied += bytes_to_write as u64;
    }
    writer.flush().expect("Could not flush writer");
}

fn dd_zeros_file(to_file: &PathBuf, n: u128) {
    let to_file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(to_file)
        .expect("Could not open file for write");
    let mut writer = BufWriter::new(to_file);
    let buf = [0; 1 << 16];
    let buf_len: u128 = buf.len() as u128;

    let mut bytes_copied: u128 = 0;
    while bytes_copied < n {
        let bytes_to_write = if bytes_copied + buf_len <= n {
            buf_len
        } else {
            n - bytes_copied
        } as usize;
        writer
            .write_all(&buf[..bytes_to_write])
            .expect("Could not write");
        bytes_copied += bytes_to_write as u128
    }
    writer.flush().expect("Could not flush writer");
}

fn config_files_already_there(
    expected_count: usize,
    expected_size: u128,
    paths: &Vec<PathBuf>,
) -> bool {
    paths
        .iter()
        .filter(|p| p.exists() && p.metadata().unwrap().len() as u128 == expected_size)
        .count()
        == expected_count
}

fn make_file_paths(file_count: u16, target_dir: &Path) -> Vec<PathBuf> {
    (0..file_count)
        .map(|i| {
            let file_name = format!("filler_{}", i);
            target_dir.join(Path::new(&file_name))
        })
        .collect()
}

fn create_files(file_count: u16, file_size: u128, target_dir: &Path, files_paths: &Vec<PathBuf>) {
    if !config_files_already_there(file_count.into(), file_size, files_paths) {
        println!("Creating target dir...");
        if target_dir.exists() {
            remove_dir_all(target_dir).expect("Could not remove existing target dir");
        }
        create_dir(target_dir).expect("Could not create target dir");

        let formatted_bytes = Byte::from_bytes(file_size.into()).get_appropriate_unit(false);
        println!(
            "Creating {} files of size {}...",
            file_count, formatted_bytes
        );
        if file_count < 1 {
            return;
        }

        println!("Writing files content...");
        files_paths.iter().for_each(|file_path| {
            write_content(file_path, file_size);
        });
    } else {
        println!("Using files from previous run");
    }
}

fn spawn_cat_forever_thread(
    files_paths: Vec<PathBuf>,
    running: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        while running.load(Ordering::SeqCst) {
            let target_file = files_paths
                .choose(&mut rand::thread_rng())
                .expect("Could not pick random file");
            cat_file(target_file);
        }
    })
}

fn spawn_dd_zeros_thread(
    files_paths: Arc<Mutex<Vec<PathBuf>>>,
    file_size: u128,
    running: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        while running.load(Ordering::SeqCst) {
            let mut locked = files_paths.lock().unwrap();
            if !locked.is_empty() {
                let target_file = locked.pop().unwrap();
                drop(locked);
                dd_zeros_file(&target_file, file_size);

                locked = files_paths.lock().unwrap();
                locked.insert(0, target_file);
            }
        }
    })
}

fn set_ctrl_c_handler(running: Arc<AtomicBool>) {
    ctrlc::set_handler(move || {
        running.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
}

fn prepare_cat_random(file_count: u16, file_size: u128, target_dir: &Path) -> Vec<PathBuf> {
    let files_paths = make_file_paths(file_count, &target_dir);
    create_files(
        file_count,
        file_size,
        &target_dir,
        &files_paths,
    );
    files_paths
}

fn prepare_dd_zeros(file_count: u16, target_dir: &Path) -> Vec<PathBuf> {
    let files_paths = make_file_paths(file_count, &target_dir);
    if target_dir.exists() {
        remove_dir_all(target_dir).expect("Could not remove existing target dir");
    }
    create_dir(target_dir).expect("Could not create target dir");
    files_paths
}

impl Mode {
    fn prepare(&self, file_count: u16, file_size: u128, target_dir: &Path) -> Vec<PathBuf> {
        match self {
            Mode::CatRandom => prepare_cat_random(file_count, file_size, target_dir),
            Mode::DdZeros => prepare_dd_zeros(file_count, target_dir),
        }
    }

    fn do_once(&self, files_paths: &Vec<PathBuf>, file_size: u128) {
        match self {
            Mode::CatRandom => {
                println!("Cat random once");
                files_paths.iter().for_each(|file_path| cat_file(file_path));
            }
            Mode::DdZeros => {
                println!("Dd zeros once");
                files_paths.iter().for_each(|file_path| dd_zeros_file(file_path, file_size));
            }
        }
    }

    fn do_forever(&self, files_paths: Vec<PathBuf>, n_threads: u16, file_size: u128, running: Arc<AtomicBool>) -> Vec::<thread::JoinHandle<()>> {
        let mut join_handles = Vec::<thread::JoinHandle<()>>::new();
        match self {
            Mode::CatRandom => {
                println!("Cat random forever");
                for _ in 0..n_threads {
                    join_handles.push(spawn_cat_forever_thread(
                        files_paths.clone(),
                        running.clone(),
                    ));
                }
                join_handles
            }
            Mode::DdZeros => {
                println!("Dd zeros forever");
                let files_paths: Arc<Mutex<Vec<PathBuf>>> = Arc::new(Mutex::new(files_paths));
                for _ in 0..n_threads {
                    join_handles.push(spawn_dd_zeros_thread(
                        files_paths.clone(),
                        file_size,
                        running.clone(),
                    ));
                }
                join_handles
            }
        }
    }
}

fn main() {
    let cli_args = CliArgs::parse();
    let target_dir = Path::new("mfiller");

    let files_paths = cli_args.mode.prepare(cli_args.file_count, cli_args.file_size, target_dir);

    let running = Arc::new(AtomicBool::new(true));

    set_ctrl_c_handler(running.clone());

    if cli_args.once {
        cli_args.mode.do_once(&files_paths, cli_args.file_size);
    } else {
        let join_handles = cli_args.mode.do_forever(files_paths, cli_args.threads, cli_args.file_size, running);
        join_handles.into_iter().for_each(|handle| {
            handle
                .join()
                .unwrap_or_else(|_| println!("thread join failure"));
        });
    }
}
