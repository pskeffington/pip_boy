//! Inert optional field relay entrypoint. No socket is opened.
#![forbid(unsafe_code)]

fn main() {
    let mut args = std::env::args().skip(1);
    match (args.next().as_deref(), args.next()) {
        (Some("status"), None) => {
            println!("rust-boy-mini-server / DISABLED / NO LISTENER / NO PEERS / NO QUEUE");
        }
        _ => {
            eprintln!("usage: rust-boy-mini-server status");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_has_no_listening_configuration() {
        // A deliberately minimal executable: status-only, no daemon lifecycle.
        assert_eq!(std::env::var("RUST_BOY_MINI_SERVER_AUTOSTART").ok(), None);
    }
}
