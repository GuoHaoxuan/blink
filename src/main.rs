mod database;
mod env;
mod fermi;
mod hxmt;
mod lightning;
mod search;
mod types;

// use chrono::prelude::*;
use rusqlite::Connection;
use types::Instance;

use database::{fail_task, finish_task, get_task, write_signal};

fn consume() {
    let hostname = hostname::get().unwrap().into_string().unwrap();
    let pid = std::process::id();
    let worker = format!("{}:{}", hostname, pid);
    let conn = Connection::open("blink.db").unwrap();
    conn.busy_timeout(std::time::Duration::from_secs(3600))
        .unwrap();
    while let Some((time, satellite, detector)) = get_task(&conn, &worker) {
        let hour: anyhow::Result<Box<dyn types::Instance>> =
            match (satellite.as_str(), detector.as_str()) {
                ("Fermi", "GBM") => fermi::Instance::from_epoch(&time)
                    .map(|inst| Box::new(inst) as Box<dyn types::Instance>),
                ("HXMT", "HE") => hxmt::Instance::from_epoch(&time)
                    .map(|inst| Box::new(inst) as Box<dyn types::Instance>),
                _ => panic!("Unknown satellite or detector"),
            };
        if let Err(e) = &hour {
            fail_task(
                &conn,
                &time,
                &satellite,
                &detector,
                anyhow::anyhow!(e.to_string()),
            );
            continue;
        }
        let results = hour.unwrap().search();
        match results {
            Ok(results) => {
                for result in results {
                    write_signal(&conn, &result);
                }
                finish_task(&conn, &time, "Fermi", "GBM");
            }
            Err(e) => {
                fail_task(
                    &conn,
                    &time,
                    &satellite,
                    &detector,
                    anyhow::anyhow!(e.to_string()),
                );
            }
        }
    }
}

// fn local_test() {
//     let epoch_str = "2022-01-01T14:51:17.919900Z";
//     let epoch: DateTime<Utc> = DateTime::parse_from_rfc3339(epoch_str)
//         .unwrap()
//         .with_timezone(&Utc);

//     let instance = hxmt::Instance::from_epoch(&epoch).unwrap();
//     let search_results = instance.search().unwrap();
//     println!("{:?}", search_results);
// }

fn main() {
    consume();
}
