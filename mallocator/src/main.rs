use anyhow::Result;
use rand::random;
use byte_unit::{Byte, ByteError};
use clap::{Args, Parser, Subcommand};
use std::collections::HashMap;
use std::io::{self, BufRead};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

struct State {
    pub tot_allocated: u128,
    pub base_size: usize,
    pub allocations: HashMap<usize, Vec<Box<[u8]>>>,
    pub binary_multiples: bool,
    pub same_value_fill: bool,
}

impl State {
    fn new() -> State {
        State {
            tot_allocated: 0,
            base_size: 1_000_000,
            allocations: HashMap::new(),
            binary_multiples: false,
            same_value_fill: false,
        }
    }

    fn print(&self) {
        let tot = Byte::from_bytes(self.tot_allocated).get_appropriate_unit(self.binary_multiples);
        let base =
            Byte::from_bytes(self.base_size as u128).get_appropriate_unit(self.binary_multiples);
        println!("Base: {}", base);
        println!("SameValueFill: {}\n", self.same_value_fill);
        println!("Tot allocated: {}", tot);
        let mut entries: Vec<(usize, usize)> = self
            .allocations
            .iter()
            .map(|(k, v)| (*k, v.len()))
            .collect();
        entries.sort_by_key(|k| k.0);
        for (size, count) in entries.iter() {
            let bytes = Byte::from_bytes(*size as u128).get_appropriate_unit(self.binary_multiples);
            println!("{} ({})", bytes, count);
        }
    }
}

#[derive(Debug, Parser)]
struct Command {
    #[command(subcommand)]
    command: Subcommands,
}

#[derive(Debug, Subcommand)]
enum Subcommands {
    Set(SetArgs),
    Alloc(AllocArgs),
    Free(FreeArgs),
    B,
    FlipSameValue,
}

trait Action {
    fn execute(&self, state: &mut State) -> Result<()>;
}

impl Command {
    fn execute(&self, state: &mut State) {
        match self.command.execute(state) {
            Ok(()) => {}
            Err(err) => println!("An error occurred while executing command.\n{}", err),
        }
    }
}

impl Action for Subcommands {
    fn execute(&self, state: &mut State) -> Result<()> {
        match self {
            Subcommands::Alloc(args) => args.execute(state),
            Subcommands::Free(args) => args.execute(state),
            Subcommands::Set(args) => args.execute(state),
            Subcommands::B => {
                state.binary_multiples = !state.binary_multiples;
                Ok(())
            },
            Subcommands::FlipSameValue => {
                state.same_value_fill = !state.same_value_fill;
                Ok(())
            }
        }
    }
}

fn parse_size_string(size: &str) -> Result<usize, ByteError> {
    Ok(Byte::from_str(size)?.get_bytes() as usize)
}

#[derive(Args, Debug)]
struct SetArgs {
    #[clap(value_parser = parse_size_string)]
    size: usize,
}

#[derive(Args, Debug)]
struct AllocArgs {
    count: usize,
}

#[derive(Args, Debug)]
struct FreeArgs {
    count: usize,
}

impl Action for SetArgs {
    fn execute(&self, state: &mut State) -> Result<()> {
        state.base_size = self.size;
        Ok(())
    }
}

impl Action for AllocArgs {
    fn execute(&self, state: &mut State) -> Result<()> {
        let allocations_container = state.allocations.entry(state.base_size).or_default();
        for _ in 0..self.count {
            let allocation = if state.same_value_fill {
                let n: u8 = random();
                vec![n; state.base_size]
            } else {
                let n: u128 = random();
                let mut allocation: Vec<u8> = vec![0u8; state.base_size];
                for i in 0..allocation.len() {
                    let value = if i % 10 == 0 {
                        random::<u8>()
                    } else {
                        let k: u8 = ((i % 4) as u8) * 8;
                        ((n >> k) & 0xff) as u8
                    };
                    allocation[i] = value;
                } 
                allocation
            };
            
            allocations_container.push(allocation.into_boxed_slice());
        }
        state.tot_allocated += state.base_size as u128 * self.count as u128;
        Ok(())
    }
}

impl Action for FreeArgs {
    fn execute(&self, state: &mut State) -> Result<()> {
        let allocations_container = state.allocations.entry(state.base_size).or_default();
        let iterations = std::cmp::min(self.count, allocations_container.len());
        let mut freed: u128 = 0;
        for _ in 0..iterations {
            if let Some(allocation) = allocations_container.pop() {
                drop(allocation);
                freed += state.base_size as u128;
            }
        }
        if allocations_container.is_empty() {
            state.allocations.remove(&state.base_size);
        }
        state.tot_allocated -= freed;
        Ok(())
    }
}

fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let mut state = State::new();
    let stdin = io::stdin();
    let mut iterator = stdin.lock().lines();

    print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
    state.print();
    let mut clear = false;

    while running.load(Ordering::SeqCst) {
        let line = iterator.next().unwrap().unwrap();
        let mut args_line = line.to_owned();
        args_line.insert_str(0, "dummy ");
        clear = match Command::try_parse_from(args_line.split(' ').into_iter()) {
            Ok(cmd) => {
                cmd.execute(&mut state);
                true
            }
            Err(err) => {
                println!("Error parsing command.\n{}", err);
                false
            }
        };
        if clear {
            print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
            state.print();
        }
    }
}
