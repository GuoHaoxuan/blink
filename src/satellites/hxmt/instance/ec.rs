use anyhow::{Context, Result};
use cached::proc_macro::cached;
use chrono::prelude::*;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::sync::LazyLock;

use crate::env::HXMT_EC_DIR;

#[derive(Clone, Copy)]
pub struct HxmtCsiEcRow {
    pub k: f64,
    pub b: f64,
    pub k_err: f64,
    pub b_err: f64,
}

#[derive(Clone, Copy)]
pub struct HxmtCsiEc {
    pub rows: [HxmtCsiEcRow; 18],
}

#[cached(result = true)]
fn hxmt_csi_ec_from_file(filename: String) -> Result<HxmtCsiEc> {
    let file =
        File::open(&filename).with_context(|| format!("Failed to open file: {}", filename))?;
    let reader = io::BufReader::new(file);
    let mut rows = [HxmtCsiEcRow {
        k: 0.0,
        b: 0.0,
        k_err: 0.0,
        b_err: 0.0,
    }; 18];
    for line in reader.lines().skip(1).take(18) {
        let line = line.with_context(|| format!("Failed to read line from file: {}", filename))?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            return Err(anyhow::anyhow!(
                "Invalid line format in file: {}. Expected at least 5 parts, got {}",
                filename,
                parts.len()
            ));
        }
        let id = parts[0]
            .parse::<u32>()
            .with_context(|| format!("Failed to parse ID from line: {}", line))?;
        let k = parts[1]
            .parse::<f64>()
            .with_context(|| format!("Failed to parse K from line: {}", line))?;
        let b = parts[2]
            .parse::<f64>()
            .with_context(|| format!("Failed to parse B from line: {}", line))?;
        let k_err = parts[3]
            .parse::<f64>()
            .with_context(|| format!("Failed to parse K error from line: {}", line))?;
        let b_err = parts[4]
            .parse::<f64>()
            .with_context(|| format!("Failed to parse B error from line: {}", line))?;
        rows[id as usize] = HxmtCsiEcRow { k, b, k_err, b_err };
    }
    Ok(HxmtCsiEc { rows })
}

static HXMT_CSI_EC_NOR_NAMES: LazyLock<Vec<String>> = LazyLock::new(|| {
    std::fs::read_dir(HXMT_EC_DIR.as_str())
        .with_context(|| format!("Failed to read directory: {}", HXMT_EC_DIR.as_str()))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .map(|path| path.file_name().unwrap().to_str().unwrap().to_string())
        .filter(|name| name.ends_with("_EC_Nor.txt"))
        .collect::<Vec<_>>()
});

impl HxmtCsiEc {
    pub fn from_file(filename: &str) -> Result<Self> {
        hxmt_csi_ec_from_file(filename.to_string())
    }

    pub fn from_datetime(datetime: &DateTime<Utc>) -> Result<Self> {
        let mut names = HXMT_CSI_EC_NOR_NAMES.clone();
        names.sort_by_key(|name| {
            let file_time = NaiveDateTime::parse_from_str(
                &(name[..10].to_string() + " 00:00:00"),
                "%Y-%m-%d %H:%M:%S",
            )
            .unwrap()
            .and_utc();
            (file_time - datetime).abs()
        });
        Self::from_file(
            Path::new(HXMT_EC_DIR.as_str())
                .join(&names[0])
                .to_str()
                .unwrap(),
        )
    }

    pub fn channel_to_energy(&self, channel: u16) -> Result<f64> {
        if channel as usize >= self.rows.len() {
            return Err(anyhow::anyhow!("Channel index out of bounds: {}", channel));
        }
        let row = self.rows[channel as usize];
        Ok(row.k * (channel as f64) + row.b)
    }
}
