use crate::types::Lightning;
use chrono::prelude::*;
use rusqlite::{Connection, params};
use std::env;
use std::sync::LazyLock;
use std::sync::Mutex;

pub static LIGHTNING_CONNECTION: LazyLock<Mutex<Connection>> = LazyLock::new(|| {
    Mutex::new({
        let path = env::var("WWLLN_DB_PATH")
            .unwrap_or_else(|_| String::from("/Volumes/Graphite/WWLLN/WWLLN.db"));
        let conn =
            Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        // Set a longer busy timeout (e.g., 30 seconds = 30000 ms)
        conn.busy_timeout(std::time::Duration::from_secs(30))
            .unwrap();
        conn
    })
});

pub fn get_lightnings(time_start: DateTime<Utc>, time_end: DateTime<Utc>) -> Vec<Lightning> {
    let time_start_str = time_start.format("%Y-%m-%d %H:%M:%S%.6f").to_string();
    let time_end_str = time_end.format("%Y-%m-%d %H:%M:%S%.6f").to_string();
    let connection = LIGHTNING_CONNECTION.lock().unwrap();
    let mut statement = connection
        .prepare(
            "
                SELECT
                    time,
                    lat,
                    lon,
                    resid,
                    nstn,
                    energy,
                    energy_uncertainty,
                    estn
                FROM
                    lightning
                WHERE
                    time BETWEEN ?1 AND ?2
                ORDER BY time ASC
                ",
        )
        .unwrap();
    statement
        .query_map(params![time_start_str, time_end_str], |row| {
            Ok(Lightning {
                time: NaiveDateTime::parse_from_str(
                    &row.get::<_, String>(0).unwrap(),
                    "%Y-%m-%d %H:%M:%S%.6f",
                )
                .unwrap()
                .and_utc(),
                lat: row.get::<_, f64>(1).unwrap(),
                lon: row.get::<_, f64>(2).unwrap(),
                resid: row.get::<_, f64>(3).unwrap(),
                nstn: row.get::<_, i64>(4).unwrap() as u32,
                energy: row.get::<_, Option<f64>>(5).unwrap(),
                energy_uncertainty: row.get::<_, Option<f64>>(6).unwrap(),
                estn: row.get::<_, Option<i64>>(7).unwrap().map(|x| x as u32),
            })
        })
        .unwrap()
        .map(|x| x.unwrap())
        .collect::<Vec<_>>()
}
