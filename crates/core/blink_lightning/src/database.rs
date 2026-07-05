use crate::types::Lightning;
use chrono::prelude::*;
use rusqlite::{Connection, Row, params};
use std::env;

thread_local! {
    // 每个线程持有独立的只读连接：SQLite 允许多读者并发，用线程本地连接（而非
    // 全局 Mutex<Connection>）让 filter 的百万级查询能真正并行，而不是串行等锁。
    static LIGHTNING_CONNECTION: Connection = open_connection();
}

fn open_connection() -> Connection {
    let path = env::var("WWLLN_DB_PATH")
        .unwrap_or_else(|_| String::from("/Volumes/Graphite/WWLLN/WWLLN.db"));
    let conn =
        Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    // Set a longer busy timeout (e.g., 30 seconds = 30000 ms)
    conn.busy_timeout(std::time::Duration::from_secs(30))
        .unwrap();
    conn
}

fn map_row(row: &Row) -> rusqlite::Result<Lightning> {
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
}

pub fn get_lightnings(time_start: DateTime<Utc>, time_end: DateTime<Utc>) -> Vec<Lightning> {
    let time_start_str = time_start.format("%Y-%m-%d %H:%M:%S%.6f").to_string();
    let time_end_str = time_end.format("%Y-%m-%d %H:%M:%S%.6f").to_string();
    LIGHTNING_CONNECTION.with(|connection| {
        let mut statement = connection
            .prepare(
                "
                SELECT time, lat, lon, resid, nstn, energy, energy_uncertainty, estn
                FROM lightning
                WHERE time BETWEEN ?1 AND ?2
                ORDER BY time ASC
                ",
            )
            .unwrap();
        statement
            .query_map(params![time_start_str, time_end_str], map_row)
            .unwrap()
            .map(|x| x.unwrap())
            .collect::<Vec<_>>()
    })
}

/// 与 `get_lightnings` 相同，但额外用一个**包含 `radius_km` 圆的**经纬度包围盒
/// 在 SQL 端预过滤。WWLLN 的 ±62s 窗会返回全球上万条闪电，其中 99% 在关联半径
/// （800km）之外；把它们挡在 SQL 层，避免为每条无关闪电在 Rust 端解析时间戳、
/// 构造 `Lightning`，是 filter 的主要提速点。
///
/// 包围盒是圆的**严格超集**（纬度余量 + 经度用盒内最靠近赤道的纬度算最宽），调用方
/// 仍做精确 haversine ≤ radius 的过滤，所以结果与仅按时间取完全一致——包围盒只影响
/// 速度。极区（盒跨极点）或跨 ±180° 经线（HXMT ±43° LEO 极少发生）时退回纯时间查询，
/// 同样不影响正确性。
pub fn get_lightnings_within(
    time_start: DateTime<Utc>,
    time_end: DateTime<Utc>,
    lat: f64,
    lon: f64,
    radius_km: f64,
) -> Vec<Lightning> {
    const KM_PER_DEG: f64 = 111.32;
    let dlat = radius_km / KM_PER_DEG + 0.5;
    let lat_min = lat - dlat;
    let lat_max = lat + dlat;

    // 经度半宽用盒内最靠近赤道的纬度（cos 最大 → 经度跨度最宽）以保证是超集。
    let min_abs_lat = (lat.abs() - dlat).max(0.0);
    let coslat = min_abs_lat.to_radians().cos();

    // 盒触及极点 / cos 太小时经度无界，退回纯时间查询。
    if !(-89.0..=89.0).contains(&lat_min) || !(-89.0..=89.0).contains(&lat_max) || coslat < 1e-3 {
        return get_lightnings(time_start, time_end);
    }
    let dlon = radius_km / (KM_PER_DEG * coslat) + 0.5;
    // 盒跨越 ±180° 经线时 BETWEEN 语义不成立，退回纯时间查询。
    if lon - dlon < -180.0 || lon + dlon > 180.0 {
        return get_lightnings(time_start, time_end);
    }
    let lon_min = lon - dlon;
    let lon_max = lon + dlon;

    let time_start_str = time_start.format("%Y-%m-%d %H:%M:%S%.6f").to_string();
    let time_end_str = time_end.format("%Y-%m-%d %H:%M:%S%.6f").to_string();
    LIGHTNING_CONNECTION.with(|connection| {
        let mut statement = connection
            .prepare(
                "
                SELECT time, lat, lon, resid, nstn, energy, energy_uncertainty, estn
                FROM lightning
                WHERE time BETWEEN ?1 AND ?2
                  AND lat BETWEEN ?3 AND ?4
                  AND lon BETWEEN ?5 AND ?6
                ORDER BY time ASC
                ",
            )
            .unwrap();
        statement
            .query_map(
                params![time_start_str, time_end_str, lat_min, lat_max, lon_min, lon_max],
                map_row,
            )
            .unwrap()
            .map(|x| x.unwrap())
            .collect::<Vec<_>>()
    })
}
