use std::{
    env,
    sync::{LazyLock, Mutex},
};

use rusqlite::Connection;

pub static GBM_DAILY_PATH: LazyLock<String> = LazyLock::new(|| {
    env::var("GBM_DAILY_PATH")
        .unwrap_or_else(|_| "/gecamfs/Exchange/GSDC/missions/FTP/fermi/data/gbm/daily".to_string())
});

pub static LIGHTNING_CONNECTION: LazyLock<Mutex<Connection>> = LazyLock::new(|| {
    Mutex::new({
        let path = env::var("WWLLN_DB_PATH")
            .unwrap_or_else(|_| String::from("/gecamfs/Exchange/GSDC/missions/AEfiles/WWLLN.db"));
        let conn =
            Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        // Set a longer busy timeout (e.g., 30 seconds = 30000 ms)
        conn.busy_timeout(std::time::Duration::from_secs(30))
            .unwrap();
        conn
    })
});

pub static HXMT_1K_DIR: LazyLock<String> = LazyLock::new(|| {
    env::var("HXMT_1K_DIR").unwrap_or_else(|_| "/hxmt/work/HXMT-DATA/1K".to_string())
});

pub static HXMT_1B_DIR: LazyLock<String> = LazyLock::new(|| {
    env::var("HXMT_1B_DIR").unwrap_or_else(|_| "/hxmtfs/data/Archive_tmp/1B".to_string())
});

pub static HXMT_EC_DIR: LazyLock<String> = LazyLock::new(|| {
    env::var("HXMT_EC_DIR")
        .unwrap_or_else(|_| "/hxmtfs2/work/GRB/Software/RSPgenerator_v1/EC_FWHM".to_string())
});

pub static HXMT_NAI_EC_FILE: LazyLock<String> = LazyLock::new(|| {
    env::var("HXMT_NAI_EC_FILE").unwrap_or_else(|_| {
        "/hxmtfs/work/users/hxmtsoft/CALDB/CALDB2.07/data/hxmt/he/bcf/hxmt_he_e2p_20190311.fits"
            .to_string()
    })
});

// Use Julian year for exact calculations
pub static DAYS_1_YEAR: f64 = 365.25;
