use anyhow::{bail, Error, Context};
use clap::{ArgGroup, Parser};
use std::fmt;
use std::io::Write;

const CONF_PATH: &str = "/proc/atomic_boost_conf";

#[derive(Debug, Clone, PartialEq)]
struct Ratio {
    pub numerator: u64,
    pub denominator: u64,
}

fn parse_ratio(s: &str) -> Result<Ratio, Error> {
    let split: Vec<&str> = s.split("/").collect();
    if split.len() != 2 {
        bail!("Ratio not well formatted");
    }
    let numerator: u64 = split[0].parse()?;
    let denominator: u64 = split[1].parse()?;
    Ok(Ratio {
        numerator,
        denominator,
    })
}

#[derive(Parser, Debug)]
#[command(group(
    ArgGroup::new("enabled")
        .args(["off", "on"])
))]
struct CliArgs {
    #[arg(long)]
    on: bool,

    #[arg(long)]
    off: bool,

    #[arg(short, long)]
    run_interval: Option<u8>,

    #[arg(short = 'i', long)]
    min_increase_interval_ns: Option<u64>,

    #[arg(short = 'd', long)]
    min_decrease_interval_ns: Option<u64>,

    #[arg(short = 's', long)]
    boost_step: Option<u64>,

    #[arg(short, long, value_parser=parse_ratio)]
    low_ratio: Option<Ratio>,

    #[arg(long)]
    reset: bool,
}

#[derive(Debug, Clone)]
struct Params {
    pub enabled: bool,
    pub run_interval: u8,
    pub min_increase_interval_ns: u64,
    pub min_decrease_interval_ns: u64,
    pub boost_step: u64,
    pub low_ratio: Ratio,
}

fn read_conf_file() -> Result<String, Error> {
    let bytes = std::process::Command::new("cat")
        .arg(CONF_PATH)
        .output()?
        .stdout;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

impl Params {
    fn read() -> Result<Params, Error> {
        Params::from_str(&read_conf_file()?)
    }

    fn from_str(s: &str) -> Result<Params, Error> {
        let split = s.split(" ").collect::<Vec<&str>>();
        if split.len() != 7 {
            bail!("Invalid format");
        };

        Ok(Params {
            enabled: split[0].parse::<u8>()? > 0,
            run_interval: split[1].parse::<u8>()?,
            min_increase_interval_ns: split[2].parse::<u64>()?,
            min_decrease_interval_ns: split[3].parse::<u64>()?,
            boost_step: split[4].parse()?,
            low_ratio: Ratio {
                numerator: split[5].parse()?,
                denominator: split[6].parse()?,
            },
        })
    }

    fn write(&self) -> Result<(), Error> {
        let mut file = std::fs::OpenOptions::new().write(true).open(CONF_PATH)?;
        writeln!(file, "{}", self.to_str()).expect("failed to write");
        Ok(())
    }

    fn reset(&self) -> Result<(), Error> {
        let mut copy = self.clone();
        copy.enabled = false;
        copy.write().context("Could not write disabled params for reset")?;
        copy.enabled = true;
        return copy.write().context("Could not write enabled params for reset");
    }

    fn to_str(&self) -> String {
        format!(
            "{} {} {} {} {} {} {}",
            if self.enabled { 1 } else { 0 },
            self.run_interval,
            self.min_increase_interval_ns,
            self.min_decrease_interval_ns,
            self.boost_step,
            self.low_ratio.numerator,
            self.low_ratio.denominator,
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

        if let Some(ratio) = &cli_args.low_ratio {
            self.low_ratio = ratio.clone();
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "1/{} runs - boost step: {} - inc/dec interval ns: {}/{} low: {}/{} {}",
            self.run_interval,
            self.boost_step,
            self.min_increase_interval_ns,
            self.min_decrease_interval_ns,
            self.low_ratio.numerator,
            self.low_ratio.denominator,
            if self.enabled { "ON" } else { "off" },
        )
    }
}

fn run(args: CliArgs) -> Result<(), Error> {
    let mut params = Params::read()?;
    if args.reset { params.reset()? }
    params.apply(&args);
    params.write()?;

    let updated_params = Params::read()?;
    Ok(println!("{:}", updated_params))
}

fn main() {
    let args = CliArgs::parse();
    if let Err(err) = run(args) {
        println!("An error occurred.\n{}", err);
    }
}
