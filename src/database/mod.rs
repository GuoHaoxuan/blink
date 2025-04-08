use chrono::prelude::*;
use rusqlite::{params, Connection};

use crate::types::Signal;

pub fn get_task(conn: &Connection, worker: &str) -> Option<(DateTime<Utc>, String, String)> {
    conn.prepare(
        "
            UPDATE tasks
            SET
                worker = ?1,
                status = 'Running',
                updated_at = DATETIME ('now')
            WHERE
                rowid IN (
                    SELECT
                        rowid
                    FROM
                        tasks
                    WHERE
                        status = 'Pending'
                    LIMIT
                        1
                ) RETURNING time, satellite, detector;
            ",
    )
    .unwrap()
    .query_map(params![worker], |row| {
        Ok((
            row.get::<_, String>(0).unwrap(),
            row.get::<_, String>(1).unwrap(),
            row.get::<_, String>(2).unwrap(),
        ))
    })
    .unwrap()
    .next()
    .map(|x| {
        let (time, satellite, detector) = x.unwrap();
        (
            NaiveDateTime::parse_from_str(&time, "%Y-%m-%d %H:%M:%S")
                .unwrap()
                .and_utc(),
            satellite,
            detector,
        )
    })
}

pub fn finish_task(conn: &Connection, time: &DateTime<Utc>, satellite: &str, detector: &str) {
    conn.execute(
        "
            UPDATE tasks
            SET
                status = 'Finished',
                updated_at = DATETIME ('now')
            WHERE
                time = ?1
                AND satellite = ?2
                AND detector = ?3;
        ",
        params![
            format!("{}", time.format("%Y-%m-%d %H:%M:%S").to_string()),
            satellite,
            detector
        ],
    )
    .unwrap();
}

pub fn fail_task(
    conn: &Connection,
    time: &DateTime<Utc>,
    satellite: &str,
    detector: &str,
    error: anyhow::Error,
) {
    conn.execute(
        "
            UPDATE tasks
            SET
                status = 'Failed',
                updated_at = DATETIME ('now'),
                retry_times = retry_times + 1,
                error = ?4
            WHERE
                time = ?1
                AND satellite = ?2
                AND detector = ?3;
        ",
        params![
            format!("{}", time.format("%Y-%m-%d %H:%M:%S").to_string()),
            satellite,
            detector,
            error.to_string()
        ],
    )
    .unwrap();
}

pub(crate) fn write_signal(conn: &Connection, signal: &Signal) {
    conn.execute(
        "
            INSERT INTO signals (start, stop, fp_year, longitude, latitude, altitude, position_debug, events, lightnings)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9);
        ",
        params![
            signal.start.to_string(),
            signal.stop.to_string(),
            signal.fp_year,
            signal.longitude,
            signal.latitude,
            signal.altitude,
            signal.position_debug,
            serde_json::to_string(&signal.events).unwrap(),
            serde_json::to_string(&signal.lightnings).unwrap(),
        ],
    )
    .unwrap();
}
