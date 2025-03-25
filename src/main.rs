mod database;
mod env;
mod fermi;
mod hxmt;
mod lightning;
mod search;
mod types;

use chrono::prelude::*;
use rusqlite::Connection;

use database::{fail_task, finish_task, get_task, write_signal};
use hxmt::Instance;

// fn consume() {
//     let hostname = hostname::get().unwrap().into_string().unwrap();
//     let pid = std::process::id();
//     let worker = format!("{}:{}", hostname, pid);
//     let conn = Connection::open("blink.db").unwrap();
//     conn.busy_timeout(std::time::Duration::from_secs(3600))
//         .unwrap();
//     while let Some(epoch) = get_task(&conn, &worker, "Fermi", "GBM") {
//         let hour = Instance::from_epoch(&epoch);
//         if let Err(e) = hour {
//             fail_task(&conn, &epoch, "Fermi", "GBM", e);
//             continue;
//         }
//         let results = hour.unwrap().search();
//         match results {
//             Ok(results) => {
//                 for result in results {
//                     write_signal(&conn, &result);
//                 }
//                 finish_task(&conn, &epoch, "Fermi", "GBM");
//             }
//             Err(e) => {
//                 fail_task(&conn, &epoch, "Fermi", "GBM", e);
//             }
//         }
//     }
// }

fn local_test() {
    let epoch_str = "2022-01-01T00:02:48.940735042Z";
    let epoch: DateTime<Utc> = DateTime::parse_from_rfc3339(epoch_str)
        .unwrap()
        .with_timezone(&Utc);

    let instance = Instance::from_epoch(&epoch).unwrap();
    println!("OK");
}

fn main() {
    local_test();
}
