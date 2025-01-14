mod database;
mod search;

use database::{fail_task, finish_task, get_task};
use rusqlite::Connection;
use search::fermi::{calculate_fermi_nai, process};

fn consume() {
    let hostname = hostname::get().unwrap().into_string().unwrap();
    let pid = std::process::id();
    let worker = format!("{}:{}", hostname, pid);
    let conn = Connection::open("blink.db").unwrap();
    conn.busy_timeout(std::time::Duration::from_secs(3600))
        .unwrap();
    while let Some(epoch) = get_task(&conn, &worker, "Fermi", "GBM") {
        let results = process(&epoch);
        match results {
            Ok(results) => {
                for result in results {
                    println!("{:?}", result);
                }
                finish_task(&conn, &epoch, "Fermi", "GBM");
            }
            Err(e) => {
                fail_task(&conn, &epoch, "Fermi", "GBM", e);
            }
        }
    }
}

fn main() {
    consume();
}
