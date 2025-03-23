use std::{
    env,
    sync::{LazyLock, Mutex},
};

use rusqlite::Connection;

pub(crate) static GBM_DAILY_PATH: LazyLock<String> = LazyLock::new(|| {
    env::var("GBM_DAILY_PATH")
        .unwrap_or_else(|_| "/gecamfs/Exchange/GSDC/missions/FTP/fermi/data/gbm/daily".to_string())
});

pub(crate) static LIGHTNING_CONNECTION: LazyLock<Mutex<Connection>> = LazyLock::new(|| {
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

pub(crate) static HXMT_1K_DIR: LazyLock<String> = LazyLock::new(|| {
    env::var("HXMT_1K_DIR").unwrap_or_else(|_| "/hxmt/work/HXMT-DATA/1K".to_string())
});

pub(crate) static HXMT_1B_DIR: LazyLock<String> =
    LazyLock::new(|| env::var("HXMT_1B_DIR").unwrap_or_else(|_| "/hxmt/data/1B".to_string()));
