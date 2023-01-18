use clap::Parser;
use rand::seq::SliceRandom;
use std::fmt;
use std::fs::{create_dir, remove_dir_all, File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
struct CliArgs {
    #[clap(parse(try_from_str = parse_size_string))]
    file_size: u64,

    #[clap()]
    file_count: u16,
}

#[derive(Debug, Clone)]
struct ParseError;
impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Invalid size")
    }
}
impl std::error::Error for ParseError {}

fn parse_size_string(size: &str) -> Result<u64, ParseError> {
    let x = size.to_uppercase();
    let multiplier = match x.chars().last().ok_or(ParseError {})? {
        'K' => 1 << 10,
        'M' => 1 << 20,
        'G' => 1 << 30,
        _ => 1,
    };
    let integer_value = x[..x.len() - 1].parse::<u64>().map_err(|_| ParseError {})?;
    Ok(integer_value * multiplier)
}

fn write_content(file_path: &PathBuf, n: u64) {
    let from_file = File::open("/dev/random").expect("Could not open /dev/random");
    let mut reader = BufReader::new(from_file);
    let to_file = OpenOptions::new()
        .write(true)
        .open(file_path)
        .expect("Could not open file for write");
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
            n as usize - bytes_read
        } as usize;
        writer
            .write_all(&buf[..bytes_to_write])
            .expect("Could not write");
        bytes_copied += bytes_to_write as u64;
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

fn create_files(count: u16, target_dir: &Path) -> Vec<PathBuf> {
    (0..count)
        .map(|i| {
            let file_name = format!("filler_{}", i);
            let file_path = target_dir.join(Path::new(&file_name));
            OpenOptions::new()
                .write(true)
                .create(true)
                .read(true)
                .truncate(true)
                .open(&file_path)
                .expect("Could not create filler");
            file_path
        })
        .collect()
}

fn main() {
    let cli_args = CliArgs::parse();

    println!("Creating target dir...");
    let target_dir = Path::new("mfiller");
    if Path::new("mfiller").exists() {
        remove_dir_all(target_dir).expect("Could not remove existing target dir");
    }
    create_dir(target_dir).expect("Could not create target dir");

    println!("Creating files...");
    let mut files_paths = create_files(cli_args.file_count, &target_dir);

    println!("Writing files content...");
    files_paths.iter_mut().for_each(|file_path| {
        write_content(file_path, cli_args.file_size);
    });

    println!("Cat forever");
    loop {
        let target_file = files_paths
            .choose(&mut rand::thread_rng())
            .expect("Could not pick random file");
        cat_file(target_file);
    }
}
