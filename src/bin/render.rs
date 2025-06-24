use blink::types::Signal;
use csv::Writer;
use indicatif::{ProgressBar, ProgressStyle};
use rusqlite::{Connection, params};
use std::fs;
use std::fs::File;

fn main() {
    // 删除旧的目录
    if fs::metadata("output").is_ok() {
        fs::remove_dir_all("output").unwrap();
    }
    // 确保目标目录存在
    fs::create_dir_all("output/detail").unwrap();

    let catalog = File::create("output/catalog.csv").unwrap();
    let mut wtr = Writer::from_writer(catalog);
    wtr.write_record([
        "start",
        "start_best",
        "stop",
        "stop_best",
        "peak",
        "duration",
        "duration_best",
        "fp_year",
        "count",
        "count_best",
        "count_filtered",
        "count_filtered_best",
        "background",
        "flux",
        "flux_best",
        "flux_filtered",
        "flux_filtered_best",
        "mean_energy",
        "mean_energy_best",
        "mean_energy_filtered",
        "mean_energy_filtered_best",
        "veto_proportion",
        "veto_proportion_best",
        "veto_proportion_filtered",
        "veto_proportion_filtered_best",
        "simultaneous_proportion",
        "simultaneous_proportion_best",
        "simultaneous_proportion_filtered",
        "simultaneous_proportion_filtered_best",
        "longitude",
        "latitude",
        "altitude",
        "q1",
        "q2",
        "q3",
        "associated_lightning_count",
        "coincidence_probability",
        "mean_solar_time",
        "apparent_solar_time",
        "day_of_year",
        "month",
        "solar_zenith_angle",
        "solar_zenith_angle_at_noon",
        "solar_azimuth_angle",
    ])
    .unwrap();

    let conn = Connection::open("blink.db").unwrap();

    // 先获取总记录数以初始化进度条
    let total_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM signal WHERE start < '2025-01-01' AND fp_year < 0.1",
            params![],
            |row| row.get(0),
        )
        .unwrap();

    // 创建进度条
    let progress_bar = ProgressBar::new(total_count as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("##-"),
    );
    progress_bar.set_message("处理信号数据");

    conn.prepare(
        "
            SELECT
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
                veto_proportion,
                veto_proportion_best,
                veto_proportion_filtered,
                veto_proportion_filtered_best,
                simultaneous_proportion,
                simultaneous_proportion_best,
                simultaneous_proportion_filtered,
                simultaneous_proportion_filtered_best,
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
                solar_azimuth_angle
            FROM signal
            WHERE start < '2025-01-01'
            AND fp_year < 0.1
            ORDER BY start
        ",
    )
    .unwrap()
    .query_map(params![], |row| {
        let start = row.get::<_, String>(0)?;
        let start_best = row.get::<_, String>(1)?;
        let stop = row.get::<_, String>(2)?;
        let stop_best = row.get::<_, String>(3)?;
        let peak = row.get::<_, String>(4)?;
        let duration = row.get::<_, f64>(5)?;
        let duration_best = row.get::<_, f64>(6)?;
        let fp_year = row.get::<_, f64>(7)?;
        let count = row.get::<_, u32>(8)?;
        let count_best = row.get::<_, u32>(9)?;
        let count_filtered = row.get::<_, u32>(10)?;
        let count_filtered_best = row.get::<_, u32>(11)?;
        let background = row.get::<_, f64>(12)?;
        let flux = row.get::<_, f64>(13)?;
        let flux_best = row.get::<_, f64>(14)?;
        let flux_filtered = row.get::<_, f64>(15)?;
        let flux_filtered_best = row.get::<_, f64>(16)?;
        let mean_energy = row.get::<_, f64>(17)?;
        let mean_energy_best = row.get::<_, f64>(18)?;
        let mean_energy_filtered = row.get::<_, f64>(19)?;
        let mean_energy_filtered_best = row.get::<_, f64>(20)?;
        let veto_proportion = row.get::<_, f64>(21)?;
        let veto_proportion_best = row.get::<_, f64>(22)?;
        let veto_proportion_filtered = row.get::<_, f64>(23)?;
        let veto_proportion_filtered_best = row.get::<_, f64>(24)?;
        let simultaneous_proportion = row.get::<_, f64>(25)?;
        let simultaneous_proportion_best = row.get::<_, f64>(26)?;
        let simultaneous_proportion_filtered = row.get::<_, f64>(27)?;
        let simultaneous_proportion_filtered_best = row.get::<_, f64>(28)?;
        let events = row.get::<_, String>(29)?;
        let light_curve_1s = row.get::<_, String>(30)?;
        let light_curve_1s_filtered = row.get::<_, String>(31)?;
        let light_curve_100ms = row.get::<_, String>(32)?;
        let light_curve_100ms_filtered = row.get::<_, String>(33)?;
        let longitude = row.get::<_, f64>(34)?;
        let latitude = row.get::<_, f64>(35)?;
        let altitude = row.get::<_, f64>(36)?;
        let q1 = row.get::<_, f64>(37)?;
        let q2 = row.get::<_, f64>(38)?;
        let q3 = row.get::<_, f64>(39)?;
        let orbit = row.get::<_, String>(40)?;
        let lightnings = row.get::<_, String>(41)?;
        let associated_lightning_count = row.get::<_, u32>(42)?;
        let coincidence_probability = row.get::<_, f64>(43)?;
        let mean_solar_time = row.get::<_, String>(44)?;
        let apparent_solar_time = row.get::<_, String>(45)?;
        let day_of_year = row.get::<_, u32>(46)?;
        let month = row.get::<_, u32>(47)?;
        let solar_zenith_angle = row.get::<_, f64>(48)?;
        let solar_zenith_angle_at_noon = row.get::<_, f64>(49)?;
        let solar_azimuth_angle = row.get::<_, f64>(50)?;
        Ok(Signal {
            start: serde_json::from_str(&format!("\"{}\"", start)).unwrap(),
            start_best: serde_json::from_str(&format!("\"{}\"", start_best)).unwrap(),
            stop: serde_json::from_str(&format!("\"{}\"", stop)).unwrap(),
            stop_best: serde_json::from_str(&format!("\"{}\"", stop_best)).unwrap(),
            peak: serde_json::from_str(&format!("\"{}\"", peak)).unwrap(),
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
            veto_proportion,
            veto_proportion_best,
            veto_proportion_filtered,
            veto_proportion_filtered_best,
            simultaneous_proportion,
            simultaneous_proportion_best,
            simultaneous_proportion_filtered,
            simultaneous_proportion_filtered_best,
            events: serde_json::from_str(&events).unwrap(),
            light_curve_1s: serde_json::from_str(&light_curve_1s).unwrap(),
            light_curve_1s_filtered: serde_json::from_str(&light_curve_1s_filtered).unwrap(),
            light_curve_100ms: serde_json::from_str(&light_curve_100ms).unwrap(),
            light_curve_100ms_filtered: serde_json::from_str(&light_curve_100ms_filtered).unwrap(),
            longitude,
            latitude,
            altitude,
            q1,
            q2,
            q3,
            orbit: serde_json::from_str(&orbit).unwrap(),
            lightnings: serde_json::from_str(&lightnings).unwrap(),
            associated_lightning_count,
            coincidence_probability,
            mean_solar_time: serde_json::from_str(&format!("\"{}\"", mean_solar_time)).unwrap(),
            apparent_solar_time: serde_json::from_str(&format!("\"{}\"", apparent_solar_time))
                .unwrap(),
            day_of_year,
            month,
            solar_zenith_angle,
            solar_zenith_angle_at_noon,
            solar_azimuth_angle,
        })
    })
    .unwrap()
    .for_each(|row| {
        let signal = row.unwrap();
        wtr.write_record([
            serde_json::to_string(&signal.start)
                .unwrap()
                .trim_matches('"')
                .to_string(),
            serde_json::to_string(&signal.start_best)
                .unwrap()
                .trim_matches('"')
                .to_string(),
            serde_json::to_string(&signal.stop)
                .unwrap()
                .trim_matches('"')
                .to_string(),
            serde_json::to_string(&signal.stop_best)
                .unwrap()
                .trim_matches('"')
                .to_string(),
            serde_json::to_string(&signal.peak)
                .unwrap()
                .trim_matches('"')
                .to_string(),
            signal.duration.to_string(),
            signal.duration_best.to_string(),
            signal.fp_year.to_string(),
            signal.count.to_string(),
            signal.count_best.to_string(),
            signal.count_filtered.to_string(),
            signal.count_filtered_best.to_string(),
            signal.background.to_string(),
            signal.flux.to_string(),
            signal.flux_best.to_string(),
            signal.flux_filtered.to_string(),
            signal.flux_filtered_best.to_string(),
            signal.mean_energy.to_string(),
            signal.mean_energy_best.to_string(),
            signal.mean_energy_filtered.to_string(),
            signal.mean_energy_filtered_best.to_string(),
            signal.veto_proportion.to_string(),
            signal.veto_proportion_best.to_string(),
            signal.veto_proportion_filtered.to_string(),
            signal.veto_proportion_filtered_best.to_string(),
            signal.simultaneous_proportion.to_string(),
            signal.simultaneous_proportion_best.to_string(),
            signal.simultaneous_proportion_filtered.to_string(),
            signal.simultaneous_proportion_filtered_best.to_string(),
            signal.longitude.to_string(),
            signal.latitude.to_string(),
            signal.altitude.to_string(),
            signal.q1.to_string(),
            signal.q2.to_string(),
            signal.q3.to_string(),
            signal.associated_lightning_count.to_string(),
            signal.coincidence_probability.to_string(),
            serde_json::to_string(&signal.mean_solar_time)
                .unwrap()
                .trim_matches('"')
                .to_string(),
            serde_json::to_string(&signal.apparent_solar_time)
                .unwrap()
                .trim_matches('"')
                .to_string(),
            signal.day_of_year.to_string(),
            signal.month.to_string(),
            signal.solar_zenith_angle.to_string(),
            signal.solar_zenith_angle_at_noon.to_string(),
            signal.solar_azimuth_angle.to_string(),
        ])
        .unwrap();
        let json_file_path = format!(
            "output/detail/{}.json",
            serde_json::to_string(&signal.start)
                .unwrap()
                .trim_matches('"')
        );
        let json_file = File::create(&json_file_path).unwrap();
        serde_json::to_writer(json_file, &signal).unwrap();

        // 更新进度条
        progress_bar.inc(1);
    });

    // 标记进度条完成
    progress_bar.finish_with_message("处理完成");
    wtr.flush().unwrap();
}
