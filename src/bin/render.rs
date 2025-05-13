use blink::types::Signal;
use csv::Writer;
use indicatif::{ProgressBar, ProgressStyle};
use rusqlite::{Connection, params};
use std::fs;
use std::fs::File;

fn main() {
    // 删除旧的目录
    if fs::metadata("detail").is_ok() {
        fs::remove_dir_all("detail").unwrap();
    }
    // 确保目标目录存在
    fs::create_dir_all("detail").unwrap();

    let catalog = File::create("catalog.csv").unwrap();
    let mut wtr = Writer::from_writer(catalog);
    wtr.write_record([
        "start",
        "stop",
        "duration",
        "start_best",
        "stop_best",
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
        "veto_ratio",
        "veto_ratio_best",
        "veto_ratio_filtered",
        "veto_ratio_filtered_best",
        "longitude",
        "latitude",
        "altitude",
        "q1",
        "q2",
        "q3",
        "associated_lightning_count",
        "coincidence_probability",
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
                stop,
                duration,
                start_best,
                stop_best,
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
                coincidence_probability
            FROM signal
            WHERE start < '2025-01-01'
            AND fp_year < 0.1
            ORDER BY start
        ",
    )
    .unwrap()
    .query_map(params![], |row| {
        let start = row.get::<_, String>(0)?;
        let stop = row.get::<_, String>(1)?;
        let duration = row.get::<_, f64>(2)?;
        let start_best = row.get::<_, String>(3)?;
        let stop_best = row.get::<_, String>(4)?;
        let duration_best = row.get::<_, f64>(5)?;
        let fp_year = row.get::<_, f64>(6)?;
        let count = row.get::<_, u32>(7)?;
        let count_best = row.get::<_, u32>(8)?;
        let count_filtered = row.get::<_, u32>(9)?;
        let count_filtered_best = row.get::<_, u32>(10)?;
        let background = row.get::<_, f64>(11)?;
        let flux = row.get::<_, f64>(12)?;
        let flux_best = row.get::<_, f64>(13)?;
        let flux_filtered = row.get::<_, f64>(14)?;
        let flux_filtered_best = row.get::<_, f64>(15)?;
        let mean_energy = row.get::<_, f64>(16)?;
        let mean_energy_best = row.get::<_, f64>(17)?;
        let mean_energy_filtered = row.get::<_, f64>(18)?;
        let mean_energy_filtered_best = row.get::<_, f64>(19)?;
        let veto_ratio = row.get::<_, f64>(20)?;
        let veto_ratio_best = row.get::<_, f64>(21)?;
        let veto_ratio_filtered = row.get::<_, f64>(22)?;
        let veto_ratio_filtered_best = row.get::<_, f64>(23)?;
        let events = row.get::<_, String>(24)?;
        let light_curve_1s = row.get::<_, String>(25)?;
        let light_curve_1s_filtered = row.get::<_, String>(26)?;
        let light_curve_100ms = row.get::<_, String>(27)?;
        let light_curve_100ms_filtered = row.get::<_, String>(28)?;
        let longitude = row.get::<_, f64>(29)?;
        let latitude = row.get::<_, f64>(30)?;
        let altitude = row.get::<_, f64>(31)?;
        let q1 = row.get::<_, f64>(32)?;
        let q2 = row.get::<_, f64>(33)?;
        let q3 = row.get::<_, f64>(34)?;
        let orbit = row.get::<_, String>(35)?;
        let lightnings = row.get::<_, String>(36)?;
        let associated_lightning_count = row.get::<_, u32>(37)?;
        let coincidence_probability = row.get::<_, f64>(38)?;
        Ok(Signal {
            start: serde_json::from_str(&start).unwrap(),
            stop: serde_json::from_str(&stop).unwrap(),
            duration,
            start_best: serde_json::from_str(&start_best).unwrap(),
            stop_best: serde_json::from_str(&stop_best).unwrap(),
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
        })
    })
    .unwrap()
    .for_each(|row| {
        let signal = row.unwrap();
        wtr.write_record([
            serde_json::to_string(&signal.start).unwrap(),
            serde_json::to_string(&signal.stop).unwrap(),
            signal.duration.to_string(),
            serde_json::to_string(&signal.start_best).unwrap(),
            serde_json::to_string(&signal.stop_best).unwrap(),
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
            signal.veto_ratio.to_string(),
            signal.veto_ratio_best.to_string(),
            signal.veto_ratio_filtered.to_string(),
            signal.veto_ratio_filtered_best.to_string(),
            signal.longitude.to_string(),
            signal.latitude.to_string(),
            signal.altitude.to_string(),
            signal.q1.to_string(),
            signal.q2.to_string(),
            signal.q3.to_string(),
            signal.associated_lightning_count.to_string(),
            signal.coincidence_probability.to_string(),
        ])
        .unwrap();
        let json_file_path = format!(
            "detail/{}.json",
            serde_json::to_string(&signal.start).unwrap()
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
