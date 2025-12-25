fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() != 4 {
        eprintln!("Usage: blink_task <detector> <total_workers> <idx_worker>");
        std::process::exit(1);
    }

    if args[1] != "HXMT/HE" {
        eprintln!("Unsupported detector: {}", args[1]);
        std::process::exit(1);
    }

    blink_task::process_all(
        args[2].parse::<usize>().expect("invalid total_workers"),
        args[3].parse::<usize>().expect("invalid idx_worker"),
    );
}
