use chrono::prelude::*;
use rusqlite::{Connection, params};

pub fn get_statistics(conn: &Connection, worker: &str) -> Option<(DateTime<Utc>, String)> {
    conn.prepare(
        "
        UPDATE statistics
        SET
            worker = ?1,
            status = 'Running',
            updated_at = DATETIME ('now')
        WHERE
            rowid IN (
                SELECT
                    rowid
                FROM
                    statistics
                WHERE
                    status = 'Pending'
                LIMIT
                    1
            ) RETURNING time, what;
        ",
    )
    .unwrap()
    .query_map(params![worker], |row| {
        Ok((
            row.get::<_, String>(0).unwrap(),
            row.get::<_, String>(1).unwrap(),
        ))
    })
    .unwrap()
    .next()
    .map(|x| {
        let (time, what) = x.unwrap();
        (
            NaiveDateTime::parse_from_str(&time, "%Y-%m-%d %H:%M:%S")
                .unwrap()
                .and_utc(),
            what,
        )
    })
}

pub fn finish_statistics(conn: &Connection, time: &DateTime<Utc>, what: &str, value: &str) {
    conn.execute(
        "
            UPDATE statistics
            SET
                status = 'Finished',
                updated_at = DATETIME ('now'),
                error = '',
                value = ?1
            WHERE
                what = ?2
                AND time = ?3;
        ",
        params![value, what, time.to_rfc3339()],
    )
    .unwrap();
}

pub fn fail_statistics(conn: &Connection, time: &DateTime<Utc>, what: &str, error: anyhow::Error) {
    conn.execute(
        "
            UPDATE statistics
            SET
                status = 'Failed',
                updated_at = DATETIME ('now'),
                error = ?1
            WHERE
                what = ?2
                AND time = ?3;
        ",
        params![error.to_string(), what, time.to_rfc3339()],
    )
    .unwrap();
}
