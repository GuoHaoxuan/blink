use rusqlite::Connection;

use blink::database::{fail_task, finish_task, get_task, write_signal};
use blink::types::Instance as BlinkInstance;

fn consume() {
    let hostname = hostname::get().unwrap().into_string().unwrap();
    let pid = std::process::id();
    let worker = format!("{}:{}", hostname, pid);
    let conn = Connection::open("blink.db").unwrap();
    conn.busy_timeout(std::time::Duration::from_secs(3600))
        .unwrap();
    while let Some((time, satellite, detector)) = get_task(&conn, &worker) {
        let hour: anyhow::Result<Box<dyn BlinkInstance>> =
            match (satellite.as_str(), detector.as_str()) {
                ("Fermi", "GBM") => blink::fermi::Instance::from_epoch(&time)
                    .map(|inst| Box::new(inst) as Box<dyn BlinkInstance>),
                ("HXMT", "HE") => blink::hxmt::Instance::from_epoch(&time)
                    .map(|inst| Box::new(inst) as Box<dyn BlinkInstance>),
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
                    write_signal(&conn, &result, &satellite, &detector);
                }
                finish_task(&conn, &time, &satellite, &detector);
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

fn main() {
    consume();
}
