use blink_hxmt_he::types::HxmtHe;
use blink_svom_grm::types::SvomGrm;
use blink_task::process_all;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() != 4 {
        eprintln!("Usage: blink_task <detector> <total_workers> <idx_worker>");
        std::process::exit(1);
    }

    let total_workers = args[2].parse::<usize>().expect("invalid total_workers");
    let idx_worker = args[3].parse::<usize>().expect("invalid idx_worker");

    match args[1].as_str() {
        "HXMT/HE" => {
            process_all::<HxmtHe>(total_workers, idx_worker);
        }
        "SVOM/GRM" => {
            process_all::<SvomGrm>(total_workers, idx_worker);
        }
        _ => {
            eprintln!("Unsupported detector: {}", args[1]);
            std::process::exit(1);
        }
    }
}
