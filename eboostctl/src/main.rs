use clap::{ArgGroup, Parser};
use std::fmt;
use std::io::Write;
use std::num::ParseIntError;

const CONF_PATH: &str = "/proc/mfkb";

#[derive(Parser, Debug)]
#[command(group(
    ArgGroup::new("enabled")
        .args(["off", "on"])
))]
struct CliArgs {
    #[clap(long)]
    on: bool,

    #[clap(long)]
    off: bool,

    #[clap(short, long)]
    run_interval: Option<u8>,

    #[clap(short = 'i', long)]
    min_increase_interval_ns: Option<u64>,

    #[clap(short = 'd', long)]
    min_decrease_interval_ns: Option<u64>,

    #[clap(short = 's', long)]
    boost_step: Option<u64>,
}

#[derive(Debug)]
struct Params {
    pub enabled: bool,
    pub run_interval: u8,
    pub min_increase_interval_ns: u64,
    pub min_decrease_interval_ns: u64,
    pub boost_step: u64,
}

#[derive(Debug)]
struct ParamsError;

impl From<ParseIntError> for ParamsError {
    fn from(_: ParseIntError) -> Self {
        ParamsError {}
    }
}

impl From<std::io::Error> for ParamsError {
    fn from(_: std::io::Error) -> Self {
        ParamsError {}
    }
}

fn read_conf_file() -> Result<String, ParamsError> {
    let bytes = std::process::Command::new("cat")
        .arg(CONF_PATH)
        .output()?
        .stdout;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

impl Params {
    fn read() -> Result<Params, ParamsError> {
        Params::from_str(&read_conf_file()?)
    }

    fn from_str(s: &str) -> Result<Params, ParamsError> {
        let split = s.split(" ").collect::<Vec<&str>>();
        if split.len() != 5 {
            return Result::Err(ParamsError {});
        };

        Ok(Params {
            enabled: split[0].parse::<u8>()? > 0,
            run_interval: split[1].parse::<u8>()?,
            min_increase_interval_ns: split[2].parse::<u64>()?,
            min_decrease_interval_ns: split[3].parse::<u64>()?,
            boost_step: split[4].parse::<u64>()?,
        })
    }

    fn write(&self) -> Result<(), ParamsError> {
        let mut file = std::fs::OpenOptions::new().write(true).open(CONF_PATH)?;
        writeln!(file, "{}", self.to_str())?;
        Ok(())
    }

    fn to_str(&self) -> String {
        format!(
            "{} {} {} {} {}",
            if self.enabled { 1 } else { 0 },
            self.run_interval,
            self.min_increase_interval_ns,
            self.min_decrease_interval_ns,
            self.boost_step,
        )
    }

    fn apply(&mut self, cli_args: &CliArgs) {
        if cli_args.on || cli_args.off {
            self.enabled = cli_args.on;
        }

        if let Some(run_interval) = cli_args.run_interval {
            self.run_interval = run_interval;
        }

        if let Some(interval) = cli_args.min_increase_interval_ns {
            self.min_increase_interval_ns = interval;
        }

        if let Some(interval) = cli_args.min_decrease_interval_ns {
            self.min_decrease_interval_ns = interval;
        }

        if let Some(step) = cli_args.boost_step {
            self.boost_step = step;
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "1/{} runs - boost step: {} - inc/dec interval ns: {}/{} {})",
            self.run_interval,
            self.boost_step,
            self.min_increase_interval_ns,
            self.min_decrease_interval_ns,
            if self.enabled { "ON" } else { "off" },
        )
    }
}

fn main() {
    let args = CliArgs::parse();
    let mut params = match Params::read() {
        Ok(x) => x,
        Err(_) => {
            println!(
                "Could not read params from {}, are you running the correct kernel?",
                CONF_PATH
            );
            return;
        }
    };
    params.apply(&args);
    println!("{:}", params);    
    params.write().expect("Could not write params.");
}
