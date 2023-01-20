use byte_unit::{Byte, ByteError};
use clap::Parser;
use rand::seq::SliceRandom;
use std::fmt;
use std::fs::{create_dir, remove_dir_all, File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
struct CliArgs {
    #[clap(short='s', long, default_value = "1M", parse(try_from_str = parse_size_string))]
    file_size: u128,

    #[clap(short = 'c', long, default_value_t = 300)]
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

fn main() {
    let cli_args = CliArgs::parse();
    let target_dir = Path::new("mfiller");

    let mut files_paths = (0..cli_args.file_count)
        .map(|i| {
            let file_name = format!("filler_{}", i);
            target_dir.join(Path::new(&file_name))
        })
        .collect();

    if !config_files_already_there(cli_args.file_count.into(), cli_args.file_size, &files_paths) {
        println!("Creating target dir...");
        if Path::new("mfiller").exists() {
            remove_dir_all(target_dir).expect("Could not remove existing target dir");
        }
        create_dir(target_dir).expect("Could not create target dir");

        let formatted_bytes =
            Byte::from_bytes(cli_args.file_size.into()).get_appropriate_unit(false);
        println!(
            "Creating {} files of size {}...",
            cli_args.file_count, formatted_bytes
        );
        if cli_args.file_count < 1 {
            return;
        }

        println!("Writing files content...");
        files_paths.iter_mut().for_each(|file_path| {
            write_content(file_path, cli_args.file_size);
        });
    } else {
        println!("Using files from previous run");
    }

    println!("Cat forever");
    loop {
        let target_file = files_paths
            .choose(&mut rand::thread_rng())
            .expect("Could not pick random file");
        cat_file(target_file);
    }
}
