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

pub fn write_signal(conn: &Connection, signal: &Signal, satellite: &str, detector: &str) {
    conn.execute(
        "
            INSERT INTO signal (
                start,
                start_best,
                stop,
                stop_best,
                peak,
                duration,
                duration_best,
                fp_year,
                count,
                count_best,
                count_filtered,
                count_filtered_best,
                background,
                flux,
                flux_best,
                flux_filtered,
                flux_filtered_best,
                mean_energy,
                mean_energy_best,
                mean_energy_filtered,
                mean_energy_filtered_best,
                veto_ratio,
                veto_ratio_best,
                veto_ratio_filtered,
                veto_ratio_filtered_best,
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
                mean_solar_time,
                apparent_solar_time,
                day_of_year,
                month,
                solar_zenith_angle,
                solar_zenith_angle_at_noon,
                solar_azimuth_angle,
                satellite,
                detector
            ) VALUES (
                 ?1,  ?2,  ?3,  ?4,  ?5,  ?6,  ?7,  ?8,  ?9, ?10,
                ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30,
                ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40,
                ?41, ?42, ?43, ?44, ?45, ?46, ?47, ?48, ?49
            );
        ",
        params![
            serde_json::to_string(&signal.start).unwrap(),
            serde_json::to_string(&signal.start_best).unwrap(),
            serde_json::to_string(&signal.stop).unwrap(),
            serde_json::to_string(&signal.stop_best).unwrap(),
            serde_json::to_string(&signal.peak).unwrap(),
            signal.duration,
            signal.duration_best,
            signal.fp_year,
            signal.count,
            signal.count_best,
            signal.count_filtered,
            signal.count_filtered_best,
            signal.background,
            signal.flux,
            signal.flux_best,
            signal.flux_filtered,
            signal.flux_filtered_best,
            signal.mean_energy,
            signal.mean_energy_best,
            signal.mean_energy_filtered,
            signal.mean_energy_filtered_best,
            signal.veto_ratio,
            signal.veto_ratio_best,
            signal.veto_ratio_filtered,
            signal.veto_ratio_filtered_best,
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
            serde_json::to_string(&signal.mean_solar_time).unwrap(),
            serde_json::to_string(&signal.apparent_solar_time).unwrap(),
            signal.day_of_year,
            signal.month,
            signal.solar_zenith_angle,
            signal.solar_zenith_angle_at_noon,
            signal.solar_azimuth_angle,
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
