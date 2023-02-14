use clap::Parser;
use libc;
use std::os::unix::io::IntoRawFd;

#[derive(Parser, Debug)]
struct CliArgs {
    #[clap(short, long, default_value_t = 2000000)]
    window_us: u64,

    #[clap(short, long, default_value_t = 150000)]
    threshold_ns: u64,

    #[clap(short, long)]
    full: bool,

    #[clap(short, long, default_value = "/proc/pressure/cpu")]
    path: String,
}

pub fn poll(fds: *mut libc::pollfd, timeout: libc::c_int) -> nix::Result<libc::c_int> {
    let res = unsafe { libc::poll(fds, 1, timeout) };

    nix::errno::Errno::result(res)
}
fn main() {
    let args = CliArgs::parse();

    println!("parsed args {:?}", args);
    let fd = std::fs::OpenOptions::new()
        .write(true)
        .open(args.path)
        .expect("Could not open proc file");

    let raw_fd: std::os::unix::io::RawFd = fd.into_raw_fd();
    let params = format!(
        "{} {} {} ",
        if args.full { "full" } else { "some" },
        args.threshold_ns,
        args.window_us
    );
    println!("wrote params {}", params);
    nix::unistd::write(raw_fd, params.as_bytes()).expect("Could not write to fd");

    let mut fds = libc::pollfd {
        fd: raw_fd,
        events: libc::POLLERR | libc::POLLPRI,
        revents: 0,
    };
    let fds_ref = &mut fds;
    loop {
        match poll(fds_ref, -1) {
            Err(x) => {
                println!("poll() failed {}", x);
                break;
            }
            Ok(n) => {
                let triggered = fds_ref.revents & libc::POLLPRI > 0;
                let error = fds_ref.revents & libc::POLLERR > 0;
                println!("poll() n {} triggered: {} error: {}", n, triggered, error);
            }
        }
    }
}
