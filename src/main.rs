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

fn local_test() {
    let filenames = [
        "2023-01-01/glg_tte_n0_230101_00z_v00.fit.gz",
        "2023-01-01/glg_tte_n1_230101_00z_v00.fit.gz",
        "2023-01-01/glg_tte_n2_230101_00z_v00.fit.gz",
        "2023-01-01/glg_tte_n3_230101_00z_v00.fit.gz",
        "2023-01-01/glg_tte_n4_230101_00z_v00.fit.gz",
        "2023-01-01/glg_tte_n5_230101_00z_v00.fit.gz",
        "2023-01-01/glg_tte_n6_230101_00z_v00.fit.gz",
        "2023-01-01/glg_tte_n7_230101_00z_v00.fit.gz",
        "2023-01-01/glg_tte_n8_230101_00z_v00.fit.gz",
        "2023-01-01/glg_tte_n9_230101_00z_v00.fit.gz",
        "2023-01-01/glg_tte_na_230101_00z_v00.fit.gz",
        "2023-01-01/glg_tte_nb_230101_00z_v00.fit.gz",
        "2023-01-01/glg_tte_b0_230101_00z_v00.fit.gz",
        "2023-01-01/glg_tte_b1_230101_00z_v00.fit.gz",
    ];
    let results = calculate_fermi_nai(&filenames);
    match results {
        Ok(results) => {
            for result in results {
                println!("{:?}", result);
            }
        }
        Err(e) => {
            println!("{:?}", e);
        }
    }
}

fn main() {
    local_test();
}
