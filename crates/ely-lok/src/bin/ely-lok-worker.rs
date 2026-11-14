use std::io;

fn main() {
    // As the app: ELY_LOG, info by default.
    env_logger::Builder::from_env(env_logger::Env::new().filter_or("ELY_LOG", "info")).init();
    let args: Vec<String> = std::env::args().collect();
    let Some(((install, profile), endpoint)) = args.get(1).zip(args.get(2)).zip(args.get(3)) else {
        log::error!("usage: ely-lok-worker <install-path> <profile-dir> <endpoint> [--idle <name>] [--dev]");
        std::process::exit(2);
    };
    let idle = match args.iter().position(|arg| arg == "--idle") {
        Some(at) => match args.get(at + 1) {
            Some(name) => Some(name.as_str()),
            None => {
                log::error!("lok-worker: --idle needs a name");
                std::process::exit(2);
            }
        },
        None => None,
    };
    let ran = ely_lok::worker_endpoint(install, profile, endpoint, idle)
        .and_then(|(input, output)| ely_lok::worker_run(install, profile, input, io::BufWriter::new(output)));
    if let Err(error) = ran {
        // A record, so the app's relay passes it on.
        log::error!("lok-worker: {error}");
        std::process::exit(1);
    }
}
