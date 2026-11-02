use std::io;

fn main() {
    // As the app: ELY_LOG, info by default.
    env_logger::Builder::from_env(env_logger::Env::new().filter_or("ELY_LOG", "info")).init();
    let args: Vec<String> = std::env::args().collect();
    let Some((install, profile)) = args.get(1).zip(args.get(2)) else {
        log::error!("usage: ely-lok-worker <install-path> <profile-dir>");
        std::process::exit(2);
    };
    #[cfg(windows)]
    let input = match ely_lok::worker_stdin() {
        Ok(pipe) => io::BufReader::new(pipe),
        Err(error) => {
            log::error!("lok-worker: {error}");
            std::process::exit(1);
        }
    };
    #[cfg(not(windows))]
    let input = io::stdin();
    let output = io::stdout();
    if let Err(error) = ely_lok::worker_run(install, profile, input, output) {
        // A record, so the app's relay passes it on.
        log::error!("lok-worker: {error}");
        std::process::exit(1);
    }
}
