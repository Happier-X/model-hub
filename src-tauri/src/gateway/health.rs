use std::{
    net::{TcpStream, ToSocketAddrs},
    time::{Duration, Instant},
};

pub fn is_reachable(host: &str, port: u16, timeout: Duration) -> bool {
    let address = format!("{host}:{port}");
    let Ok(mut addrs) = address.to_socket_addrs() else {
        return false;
    };
    let Some(addr) = addrs.next() else {
        return false;
    };
    TcpStream::connect_timeout(&addr, timeout).is_ok()
}

pub fn wait_until_reachable(
    host: &str,
    port: u16,
    overall_timeout: Duration,
    poll_interval: Duration,
) -> bool {
    let started = Instant::now();
    while started.elapsed() < overall_timeout {
        if is_reachable(host, port, Duration::from_millis(250)) {
            return true;
        }
        std::thread::sleep(poll_interval);
    }
    false
}

#[cfg(test)]
mod tests {
    use std::{net::TcpListener, time::Duration};

    use super::{is_reachable, wait_until_reachable};

    #[test]
    fn detects_open_and_closed_ports() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(is_reachable("127.0.0.1", port, Duration::from_millis(200)));
        drop(listener);
        // Port may still be in TIME_WAIT on some systems; closed check is best-effort.
        let _ = wait_until_reachable(
            "127.0.0.1",
            1,
            Duration::from_millis(50),
            Duration::from_millis(10),
        );
    }
}
