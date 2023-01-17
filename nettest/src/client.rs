use clap::Parser;
use std::net::UdpSocket;
use std::thread;
use std::time::Duration;

mod common;

use common::*;

#[derive(Parser)]
struct CliArgs {
    #[clap(short, long, default_value_t = std::u64::MAX)]
    count: u64,

    #[clap(short = 'u', long, default_value_t = 6666u16)]
    port: u16,

    #[clap(short, long)]
    sleep_time_nano: Option<u64>,
}

struct Socket {
    _socket: UdpSocket,
    _dst: String,
}

impl Socket {
    fn new(listen_port: u16, dst_port: u16) -> Socket {
        let addr = format!("0.0.0.0:{}", listen_port);
        let _socket = UdpSocket::bind(addr).expect("Could not bind socket");
        let _dst = format!("127.0.0.1:{}", dst_port);
        Socket { _socket, _dst }
    }

    fn send(&self, msg: &[u8]) {
        self._socket
            .send_to(msg, &self._dst)
            .expect("Could not send message");
    }
}

fn main() {
    let cli_args = CliArgs::parse();

    let socket = Socket::new(5689, cli_args.port);

    let msg = [49u8; 65000];

    socket.send(MSG_SEQ_START);
    for _ in 0..cli_args.count {
        socket.send(&msg);
        if let Some(sleep_us) = cli_args.sleep_time_nano {
            thread::sleep(Duration::from_nanos(sleep_us));
        }
    }
    thread::sleep(Duration::from_secs(2));

    socket.send(MSG_SEQ_END);
}
