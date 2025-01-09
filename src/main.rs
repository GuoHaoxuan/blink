mod search;

use clap::{arg, Arg, Command};
use rand::seq::IteratorRandom;
use rusqlite::{params, Connection, Result};
use search::{fermi::process, record::Record};
use std::str::FromStr;

use hifitime::prelude::*;
use polars::prelude::*;

fn print_results(results: &[Record]) {
    let df: DataFrame = df!(
        "start" => results.iter().map(|x| x.start.to_string()).collect::<Vec<_>>(),
        "stop" => results.iter().map(|x| x.stop.to_string()).collect::<Vec<_>>(),
        "bin_size_min" => results.iter().map(|x| (x.bin_size_min.total_nanoseconds() / 1000) as u32).collect::<Vec<_>>(),
        "bin_size_max" => results.iter().map(|x| (x.bin_size_max.total_nanoseconds() / 1000) as u32).collect::<Vec<_>>(),
        "bin_size_best" => results.iter().map(|x| (x.bin_size_best.total_nanoseconds() / 1000) as u32).collect::<Vec<_>>(),
        "delay" => results.iter().map(|x| (x.delay.total_nanoseconds() / 1000) as u32).collect::<Vec<_>>(),
        "count" => results.iter().map(|x| x.count).collect::<Vec<_>>(),
        "average" => results.iter().map(|x| x.average).collect::<Vec<_>>(),
    )
    .unwrap();
    if df.height() > 0 {
        println!("{}", df);
    }
}

fn product() {
    let conn = Connection::open("blink.db").unwrap();
    let start = Epoch::from_str("2023-01-01T00:00:00").unwrap();
    let end = Epoch::from_str("2024-01-01T00:00:00").unwrap();
    let step = 1.hours();
    let time_series = TimeSeries::inclusive(start, end, step);
    for epoch in time_series {
        conn.execute(
            "
                INSERT INTO
                    tasks (
                        created_at,
                        updated_at,
                        retry_times,
                        lock_id,
                        time_hour,
                        satellite
                    )
                VALUES
                    (?1, ?2, ?3, ?4, ?5, ?6);
        ",
            (
                Epoch::now().unwrap().to_string(),
                Epoch::now().unwrap().to_string(),
                0,
                "",
                epoch.to_string(),
                "fermi",
            ),
        )
        .unwrap();
    }
}

fn consume() {
    let hostname = hostname::get().unwrap().into_string().unwrap();
    let pid = std::process::id();
    let lock_id = format!("{}-{}", hostname, pid);
    let conn = Connection::open("blink.db").unwrap();
    conn.busy_timeout(std::time::Duration::from_secs(3600))
        .unwrap();
    loop {
        let mut stmt = conn
            .prepare(
                "SELECT rowid FROM tasks WHERE lock_id = '' ORDER BY rowid ASC, retry_times ASC LIMIT 100;",
            ).unwrap();
        let task_id = stmt
            .query_map([], |row| row.get::<_, i64>(0))
            .unwrap()
            .map(|x| x.unwrap())
            .choose(&mut rand::thread_rng());
        if task_id.is_none() {
            return;
        }
        let task_id = task_id.unwrap();
        conn.execute(
            "UPDATE tasks set lock_id = ?1, updated_at = ?2 where rowid = ?3;",
            params![&lock_id, Epoch::now().unwrap().to_string(), task_id],
        )
        .unwrap();
        let mut stmt = conn
            .prepare("SELECT time_hour, satellite FROM tasks WHERE rowid = ?1;")
            .unwrap();
        let tasks = stmt
            .query_map([task_id], |row| {
                Ok([row.get::<_, String>(0), row.get::<_, String>(1)].map(|x| x.unwrap()))
            })
            .unwrap()
            .map(|x| x.unwrap())
            .collect::<Vec<_>>();
        if tasks.len() != 1 {
            continue;
        }
        let task = tasks[0].clone();
        println!("Processing task: {:?}", task);
        let records = process(&Epoch::from_str(&task[0]).unwrap());
        let result = records
            .iter()
            .map(|record| record.save(&conn))
            .all(|x| x.is_ok());
        if result {
            conn.execute("DELETE FROM tasks WHERE rowid = ?1;", params![task_id])
                .unwrap();
        } else {
            conn.execute(
                "UPDATE tasks SET lock_id = '', retry_times = retry_times + 1, updated_at = ?1 WHERE rowid = ?2;",
                params![Epoch::now().unwrap().to_string(), task_id],
            ).unwrap();
        }
    }
}

fn main() {
    product();
}
