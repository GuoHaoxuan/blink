mod database;
mod env;
mod fermi;
mod hxmt;
mod lightning;
mod search;
mod types;

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
//     let eng_data = hxmt::EngFile::new("HXMT_1B_0766_20200428T140000_G025146_000_004.fits").unwrap();
//     let sci_data = hxmt::SciFile::new("HXMT_1B_0642_20200428T140000_G025146_000_004.fits").unwrap();
//     let epoch_str = "2020-04-28T14:34:24.000000Z";
//     let epoch: Time<Hxmt> = DateTime::parse_from_rfc3339(epoch_str)
//         .unwrap()
//         .with_timezone(&Utc)
//         .into();

//     let saturation = rec_sci_data(epoch, &eng_data, &sci_data);
//     println!("Saturation: {}", saturation);
// }

fn main() {
    consume();
}
