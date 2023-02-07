use anyhow::bail;
use anyhow::Error;
use byte_unit::{Byte, ByteError};
use clap::Parser;
use std::io::Write;
use std::thread;
use std::{fmt, fs};

const CONF_PATH: &str = "/proc/kmallocer/conf";
const DO_PATH: &str = "/proc/kmallocer/do";

#[derive(Parser, Debug)]
struct CliArgs {
    #[arg(short, long)]
    iterations: Option<u16>,

    #[arg(short = 't', long)]
    max_iteration_time_ms: Option<u64>,

    #[arg(short, long, value_parser = parse_size_string)]
    max_allocation: Option<u128>,

    #[arg(short = 'w', long)]
    hold_time_ms: Option<u64>,

    #[arg(long = "min-o")]
    min_allocation_size_order: Option<u8>,

    #[arg(long = "max-o")]
    max_allocation_size_order: Option<u8>,

    #[arg(short = 'j', long, default_value_t = 1)]
    threads: u8,
}

fn parse_size_string(size: &str) -> Result<u128, ByteError> {
    Ok(Byte::from_str(size)?.get_bytes())
}

#[derive(Debug)]
struct Params {
    pub iterations: u16,
    pub max_iteration_time_ms: u64,
    pub max_allocation: u128,
    pub hold_time_ms: u64,
    pub min_allocation_size: u64,
    pub max_allocation_size: u64,
}

fn cat_file(path: &str) -> Result<String, Error> {
    let bytes = std::process::Command::new("cat").arg(path).output()?.stdout;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} iter - max {} ms - target alloc {} - hold time {} ms - min/max all {}/{}",
            self.iterations,
            self.max_iteration_time_ms,
            self.max_allocation,
            self.hold_time_ms,
            self.min_allocation_size,
            self.max_allocation_size,
        )
    }
}

impl Params {
    fn read() -> Result<Params, Error> {
        Params::from_str(&cat_file(CONF_PATH)?)
    }

    fn from_str(s: &str) -> Result<Params, Error> {
        let split = s.split(" ").collect::<Vec<&str>>();
        if split.len() != 6 {
            bail!("Invalid format");
        };

        Ok(Params {
            iterations: split[0].parse()?,
            max_allocation: split[1].parse()?,
            min_allocation_size: split[2].parse()?,
            max_allocation_size: split[3].parse()?,
            max_iteration_time_ms: split[4].parse::<u64>()? / 1_000_000u64,
            hold_time_ms: split[5].parse()?,
        })
    }

    fn write(&self) -> Result<(), Error> {
        let mut file = fs::OpenOptions::new().write(true).open(CONF_PATH)?;
        writeln!(file, "{}", self.to_str())?;
        Ok(())
    }

    fn to_str(&self) -> String {
        format!(
            "{} {} {} {} {} {}",
            self.iterations,
            self.max_allocation,
            self.min_allocation_size,
            self.max_allocation_size,
            self.max_iteration_time_ms * 1_000_000,
            self.hold_time_ms,
        )
    }

    fn apply(&mut self, cli_args: &CliArgs) {
        if let Some(iterations) = cli_args.iterations {
            self.iterations = iterations;
        }

        if let Some(max_iteration_time_ms) = cli_args.max_iteration_time_ms {
            self.max_iteration_time_ms = max_iteration_time_ms;
        }

        if let Some(max_allocation) = cli_args.max_allocation {
            self.max_allocation = max_allocation;
        }

        if let Some(hold_time_ms) = cli_args.hold_time_ms {
            self.hold_time_ms = hold_time_ms;
        }

        if let Some(max_order) = cli_args.max_allocation_size_order {
            self.max_allocation_size = 1 << (12 + max_order);
        }

        if let Some(min_order) = cli_args.min_allocation_size_order {
            self.min_allocation_size = 1 << (12 + min_order);
        }
    }
}

fn run(threads: u8) -> Result<(), Error> {
    let mut join_handles: Vec<thread::JoinHandle<Result<String, Error>>> = Vec::new();
    for _ in 0..threads {
        let handle = thread::spawn(|| cat_file(DO_PATH));
        join_handles.push(handle);
    }

    let mut error = false;
    let mut results: Vec<String> = Vec::new();
    join_handles.into_iter().for_each(|handle| {
        let res = handle.join();
        match res {
            Ok(Ok(s)) => results.push(s),
            _ => error = true,
        }
    });
    results.iter().for_each(|result| {
        println!("{}", result);
    });
    if error {
        bail!("Something went wrong");
    }
    Ok(())
}

fn print_read_error_msg() {
    println!(
        "Could nto read params from {}, did you load kmallocer module?",
        CONF_PATH
    );
}

fn main() {
    let args = CliArgs::parse();
    let mut params = match Params::read() {
        Ok(x) => x,
        Err(_) => return print_read_error_msg(),
    };
    println!("Read params: {}", params);
    params.apply(&args);

    let _ = params.write();

    let updated_params = match Params::read() {
        Ok(x) => x,
        Err(_) => return print_read_error_msg(),
    };
    println!("Updated params: {}", updated_params);

    if let Err(x) = run(args.threads) {
        println!("Error while running kmallocer.\n {}", x);
    }
}
