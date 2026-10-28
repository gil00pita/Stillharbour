fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    if (3..=4).contains(&args.len()) && args[1] == "--media-worker-stdin" {
        let result = stillharbour_core::media::worker_stdin_arguments(
            &args[2],
            args.get(3).map(|s| s.as_os_str()),
        );
        if result.is_err() {
            eprintln!("Preview unavailable; original unchanged.");
            std::process::exit(1);
        }
        return;
    }
    if args.len() != 3 || args[1] != "--media-worker" {
        std::process::exit(2);
    }
    if stillharbour_core::media::worker_main(std::path::Path::new(&args[2])).is_err() {
        eprintln!("Media worker failed; input was preserved.");
        std::process::exit(1);
    }
}
