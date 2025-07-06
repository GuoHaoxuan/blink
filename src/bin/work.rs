use rusqlite::Connection;

use blink::database::{fail_task, finish_task, get_task, write_signal};
use blink::satellites::hxmt::types::HxmtScintillator;
use blink::types::{Event, Instance as BlinkInstance};

fn consume() {
    let hostname = hostname::get().unwrap().into_string().unwrap();
    let pid = std::process::id();
    let worker = format!("{}:{}", hostname, pid);
    let conn = Connection::open("blink.db").unwrap();
    conn.busy_timeout(std::time::Duration::from_secs(3600))
        .unwrap();

    // consume tasks
    while let Some((time, satellite, detector)) = get_task(&conn, &worker) {
        let hour: anyhow::Result<Box<dyn BlinkInstance>> =
            match (satellite.as_str(), detector.as_str()) {
                ("Fermi", "GBM") => blink::satellites::fermi::Instance::from_epoch(&time)
                    .map(|inst| Box::new(inst) as Box<dyn BlinkInstance>),
                ("HXMT", "HE") => blink::satellites::hxmt::Instance::from_epoch(&time)
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

    // consume statistics
    while let Some((time, what)) = blink::database::get_statistics(&conn, &worker) {
        let result = match what.as_str() {
            "HXMT-HE: Energy Spectrum" => {
                let mut result = vec![0u64; 256 + 20];
                let instance = blink::satellites::hxmt::Instance::from_epoch(&time);
                match instance {
                    Ok(instance) => {
                        for channel in instance
                            .into_iter()
                            .filter(|event| !event.detector.am241)
                            .filter(|event| event.detector.scintillator == HxmtScintillator::CsI)
                            .map(|event| event.channel())
                        {
                            result[channel as usize] += 1;
                        }
                        Ok(serde_json::to_value(result).unwrap().to_string())
                    }
                    Err(e) => Err(anyhow::anyhow!(e.to_string())),
                }
            }
            _ => panic!("Unknown statistics type"),
        };
        match result {
            Ok(value) => {
                blink::database::finish_statistics(&conn, &time, &what, &value);
            }
            Err(e) => {
                blink::database::fail_statistics(&conn, &time, &what, e);
            }
        }
    }
}

fn main() {
    consume();
}
