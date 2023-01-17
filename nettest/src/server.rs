use clap::Parser;
use std::net::UdpSocket;

mod common;

use common::*;

#[derive(Parser)]
struct CliArgs {
    #[clap(short = 'u', long, default_value_t = 6667u16)]
    port: u16,
}

const TAG: &str = "[NETTEST SERVER]";

fn log<T: AsRef<str> + std::fmt::Display>(msg: T) {
    println!("{}: {}", TAG, msg);
}

fn main() {

    let cli_args = CliArgs::parse();

    log("started");
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", cli_args.port)).unwrap();
    log("bounded");

    let mut buf = [0u8; 1024];
    let mut rcv_count = 0;

    loop {
        let (amt, _src) = socket
            .recv_from(&mut buf)
            .expect("Could not receive message");

        match &buf[..amt] {
            MSG_SEQ_START => {
                log("received SEQ_START");
                rcv_count = 0;
            }
            MSG_SEQ_END => {
                log("received SEQ_END");                
                log(format!("{}", rcv_count));
            }
            _ => {
                rcv_count += 1;
            }
        }
    }
}
