mod database;
mod search;

use database::get_task;
use rusqlite::Connection;
use search::fermi::process;

fn consume() {
    let hostname = hostname::get().unwrap().into_string().unwrap();
    let pid = std::process::id();
    let worker = format!("{}:{}", hostname, pid);
    let conn = Connection::open("blink.db").unwrap();
    conn.busy_timeout(std::time::Duration::from_secs(3600))
        .unwrap();
    while let Some(epoch) = get_task(&conn, &worker, "Fermi", "GBM") {
        let results = process(&epoch);
        results.iter().for_each(|x| x.save(&conn).unwrap());
    }
}

fn main() {
    consume();
}
