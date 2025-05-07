use chrono::prelude::*;
use rusqlite::{params, Connection};

use crate::types::Signal;

pub fn get_task(conn: &Connection, worker: &str) -> Option<(DateTime<Utc>, String, String)> {
    conn.prepare(
        "
            UPDATE task
            SET
                worker = ?1,
                status = 'Running',
                updated_at = DATETIME ('now')
            WHERE
                rowid IN (
                    SELECT
                        rowid
                    FROM
                        task
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
            UPDATE task
            SET
                status = 'Finished',
                updated_at = DATETIME ('now'),
                error = ''
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
            UPDATE task
            SET
                status = 'Failed',
                updated_at = DATETIME ('now'),
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
            format!("{:#}", error),
        ],
    )
    .unwrap();
}

pub(crate) fn write_signal(conn: &Connection, signal: &Signal, satellite: &str, detector: &str) {
    conn.execute(
        "
            INSERT INTO signal (
                start,
                stop,
                best_start,
                best_stop,
                count,
                best_count,
                count_all,
                fp_year,
                background,
                events,
                light_curve_1s,
                light_curve_1s_filtered,
                light_curve_100ms,
                light_curve_100ms_filtered,
                longitude,
                latitude,
                altitude,
                q1,
                q2,
                q3,
                orbit,
                lightnings,
                associated_lightning_count,
                coincidence_probability,
                satellite,
                detector
            ) VALUES (
                 ?1,  ?2,  ?3,  ?4,  ?5,  ?6,  ?7,  ?8,  ?9, ?10,
                ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                ?21, ?22, ?23, ?24, ?25, ?26
            );
        ",
        params![
            signal.start.format("%Y-%m-%dT%H:%M:%S%.9f%:z").to_string(),
            signal.stop.format("%Y-%m-%dT%H:%M:%S%.9f%:z").to_string(),
            signal
                .best_start
                .format("%Y-%m-%dT%H:%M:%S%.9f%:z")
                .to_string(),
            signal
                .best_stop
                .format("%Y-%m-%dT%H:%M:%S%.9f%:z")
                .to_string(),
            signal.count,
            signal.best_count,
            signal.count_all,
            signal.fp_year,
            signal.background,
            serde_json::to_string(&signal.events).unwrap(),
            serde_json::to_string(&signal.light_curve_1s).unwrap(),
            serde_json::to_string(&signal.light_curve_1s_filtered).unwrap(),
            serde_json::to_string(&signal.light_curve_100ms).unwrap(),
            serde_json::to_string(&signal.light_curve_100ms_filtered).unwrap(),
            signal.longitude,
            signal.latitude,
            signal.altitude,
            signal.q1,
            signal.q2,
            signal.q3,
            serde_json::to_string(&signal.orbit).unwrap(),
            serde_json::to_string(&signal.lightnings).unwrap(),
            signal.associated_lightning_count,
            signal.coincidence_probability,
            satellite,
            detector,
        ],
    )
    .inspect_err(|e| {
        eprintln!("Error writing signal to database: {}", e);
        eprintln!("Signal: {:?}", signal);
    })
    .unwrap();
}
