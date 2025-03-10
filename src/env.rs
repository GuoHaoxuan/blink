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
    Mutex::new(
        Connection::open_with_flags(
            env::var("WWLLN_DB_PATH").unwrap_or_else(|_| {
                String::from("/gecamfs/Exchange/GSDC/missions/AEfiles/WWLLN.db")
            }),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap(),
    )
});
