use hifitime::prelude::*;
use rusqlite::{params, Connection};
use std::str::FromStr;

pub fn get_task(conn: &Connection, worker: &str, satellite: &str, detector: &str) -> Option<Epoch> {
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
                        AND satellite = ?2
                        AND detector = ?3
                    LIMIT
                        1
                ) RETURNING time;
            ",
    )
    .unwrap()
    .query_map(params![worker, satellite, detector], |row| {
        row.get::<_, String>(0)
    })
    .unwrap()
    .next()
    .map(|x| Epoch::from_str(&x.unwrap()).unwrap())
}

pub fn finish_task(conn: &Connection, time: &Epoch, satellite: &str, detector: &str) {
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
            format!(
                "{}",
                Formatter::new(*time, Format::from_str("%Y-%m-%d %H:%M:%S").unwrap())
            ),
            satellite,
            detector
        ],
    )
    .unwrap();
}

pub fn fail_task(conn: &Connection, time: &Epoch, satellite: &str, detector: &str) {
    conn.execute(
        "
            UPDATE tasks
            SET
                status = 'Failed',
                updated_at = DATETIME ('now'),
                retry_times = retry_times + 1
            WHERE
                time = ?1
                AND satellite = ?2
                AND detector = ?3;
        ",
        params![
            format!(
                "{}",
                Formatter::new(*time, Format::from_str("%Y-%m-%d %H:%M:%S").unwrap())
            ),
            satellite,
            detector
        ],
    )
    .unwrap();
}
