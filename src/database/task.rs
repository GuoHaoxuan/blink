use chrono::prelude::*;
use rusqlite::{Connection, params};

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
            ) RETURNING satellite, detector, time;
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
        let (satellite, detector, time) = x.unwrap();
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
                satellite = ?1
                AND detector = ?2
                AND time = ?3;
        ",
        params![
            satellite,
            detector,
            format!("{}", time.format("%Y-%m-%d %H:%M:%S").to_string())
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
                error = ?1
            WHERE
                satellite = ?2
                AND detector = ?3
                AND time = ?4;
        ",
        params![
            format!("{:#}", error),
            satellite,
            detector,
            format!("{}", time.format("%Y-%m-%d %H:%M:%S").to_string())
        ],
    )
    .unwrap();
}

pub fn write_signal(conn: &Connection, signal: &Signal, satellite: &str, detector: &str) {
    conn.execute(
        "
            INSERT INTO signal (
                satellite,
                detector,
                start_full,
                start_best,
                stop_full,
                stop_best,
                peak,
                duration_full,
                duration_best,
                false_positive,
                false_positive_per_year,
                count_unfiltered_full,
                count_unfiltered_best,
                count_filtered_full,
                count_filtered_best,
                background,
                flux_unfiltered_full,
                flux_unfiltered_best,
                flux_filtered_full,
                flux_filtered_best,
                events,
                light_curve_1s_unfiltered,
                light_curve_1s_filtered,
                light_curve_100ms_unfiltered,
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
                mean_solar_time,
                apparent_solar_time,
                day_of_year,
                month,
                solar_zenith_angle,
                solar_zenith_angle_at_noon,
                solar_azimuth_angle
            ) VALUES (
                 ?1,  ?2,  ?3,  ?4,  ?5,  ?6,  ?7,  ?8,  ?9, ?10,
                ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30,
                ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40,
                ?41, ?42
            );
        ",
        params![
            satellite,
            detector,
            serde_json::to_string(&signal.start_full)
                .unwrap()
                .trim_matches('"'),
            serde_json::to_string(&signal.start_best)
                .unwrap()
                .trim_matches('"'),
            serde_json::to_string(&signal.stop_full)
                .unwrap()
                .trim_matches('"'),
            serde_json::to_string(&signal.stop_best)
                .unwrap()
                .trim_matches('"'),
            serde_json::to_string(&signal.peak)
                .unwrap()
                .trim_matches('"'),
            signal.duration_full,
            signal.duration_best,
            signal.false_positive,
            signal.false_positive_per_year,
            signal.count_unfiltered_full,
            signal.count_unfiltered_best,
            signal.count_filtered_full,
            signal.count_filtered_best,
            signal.background,
            signal.flux_unfiltered_full,
            signal.flux_unfiltered_best,
            signal.flux_filtered_full,
            signal.flux_filtered_best,
            serde_json::to_string(&signal.events).unwrap(),
            serde_json::to_string(&signal.light_curve_1s_unfiltered).unwrap(),
            serde_json::to_string(&signal.light_curve_1s_filtered).unwrap(),
            serde_json::to_string(&signal.light_curve_100ms_unfiltered).unwrap(),
            serde_json::to_string(&signal.light_curve_100ms_filtered).unwrap(),
            signal.longitude,
            signal.latitude,
            signal.altitude,
            signal.q1,
            signal.q2,
            signal.q3,
            serde_json::to_string(&signal.orbit.data).unwrap(),
            serde_json::to_string(&signal.lightnings).unwrap(),
            signal.associated_lightning_count,
            signal.coincidence_probability,
            serde_json::to_string(&signal.mean_solar_time)
                .unwrap()
                .trim_matches('"'),
            serde_json::to_string(&signal.apparent_solar_time)
                .unwrap()
                .trim_matches('"'),
            signal.day_of_year,
            signal.month,
            signal.solar_zenith_angle,
            signal.solar_zenith_angle_at_noon,
            signal.solar_azimuth_angle
        ],
    )
    .inspect_err(|e| {
        eprintln!("Error writing signal to database: {}", e);
        eprintln!("Signal: {:?}", signal);
    })
    .unwrap();
}
