use anyhow::{Context, Result};
use chrono::prelude::*;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use crate::env::HXMT_EC_DIR;

#[derive(Clone, Copy)]
pub struct HxmtEcRow {
    pub k: f64,
    pub b: f64,
    pub k_err: f64,
    pub b_err: f64,
}

#[derive(Clone, Copy)]
pub struct HxmtEc {
    pub rows: [HxmtEcRow; 18],
}

impl HxmtEc {
    pub fn from_file(filename: &str) -> Result<Self> {
        let file =
            File::open(filename).with_context(|| format!("Failed to open file: {}", filename))?;
        let reader = io::BufReader::new(file);
        let mut rows = [HxmtEcRow {
            k: 0.0,
            b: 0.0,
            k_err: 0.0,
            b_err: 0.0,
        }; 18];
        for line in reader.lines().skip(1).take(18) {
            let line =
                line.with_context(|| format!("Failed to read line from file: {}", filename))?;
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
            rows[id as usize] = HxmtEcRow { k, b, k_err, b_err };
        }
        Ok(HxmtEc { rows })
    }

    pub fn from_datetime(datetime: &DateTime<Utc>) -> Result<Self> {
        let mut names = std::fs::read_dir(HXMT_EC_DIR.as_str())
            .with_context(|| format!("Failed to read directory: {}", HXMT_EC_DIR.as_str()))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .map(|path| path.file_name().unwrap().to_str().unwrap().to_string())
            .filter(|name| name.ends_with("_EC_Nor.txt"))
            .collect::<Vec<_>>();
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
}
