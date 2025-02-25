use hifitime::prelude::*;
use itertools::Itertools;
use regex::Regex;
use std::error::Error;

use crate::fermi::{Detector, Event, Group};
use crate::types::Interval;

use super::algorithms::{search, SearchConfig};

pub fn calculate_fermi(
    filenames: &[(&str, Detector)],
) -> Result<Vec<Interval<Epoch>>, Box<dyn Error>> {
    let group = Group::new(filenames)?;
    let events: Vec<Event> = group
        .into_iter()
        .dedup_by_with_count(|a, b| b.time() - a.time() < 0.3e-6.seconds())
        .filter(|(count, _)| *count == 1)
        .map(|(_, event)| event)
        .filter(|event| event.pha() >= 30 && event.pha() <= 124)
        .collect();
    let gti = group.gti();

    Ok(gti
        .into_iter()
        .flat_map(|interval| {
            search(
                &events,
                6,
                interval.start,
                interval.stop,
                SearchConfig {
                    ..Default::default()
                },
            )
        })
        .collect())
}

fn get_fermi_filenames(epoch: &Epoch) -> Result<Vec<(String, Detector)>, Box<dyn Error>> {
    let (y, m, d, h, ..) = epoch.to_gregorian_utc();
    let folder = format!(
        "/gecamfs/Exchange/GSDC/missions/FTP/fermi/data/gbm/daily/{:04}/{:02}/{:02}/current",
        y, m, d
    );
    let mut filenames = Vec::new();
    for i in 0..12 {
        let pattern = format!(
            "glg_tte_{}_{:02}{:02}{:02}_{:02}z_v\\d{{2}}\\.fit\\.gz",
            if i < 12 {
                format!("n{:x}", i)
            } else {
                format!("b{:x}", i - 12)
            },
            y % 100,
            m,
            d,
            h,
        );
        let detector = if i < 12 {
            Detector::Nai(i)
        } else {
            Detector::Bgo(i - 12)
        };
        let re = Regex::new(&pattern)?;
        let max_file = std::fs::read_dir(&folder)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .filter_map(|path| {
                path.file_name()
                    .and_then(|name| name.to_str().map(String::from))
            })
            .filter(|name| re.is_match(name))
            .max_by(|a, b| {
                let extract_version = |name: &str| {
                    name.split('_')
                        .last()
                        .and_then(|s| s.strip_prefix('v'))
                        .and_then(|s| s.strip_suffix(".fit.gz"))
                        .and_then(|s| s.parse::<u32>().ok())
                        .unwrap_or(0)
                };
                extract_version(a).cmp(&extract_version(b))
            });
        if let Some(file) = max_file {
            filenames.push((format!("{}/{}", folder, file), detector));
        }
    }
    if filenames.is_empty() {
        return Err("No matching files found".into());
    }
    Ok(filenames)
}

pub fn process(epoch: &Epoch) -> Result<Vec<Interval<Epoch>>, Box<dyn Error>> {
    let filenames = get_fermi_filenames(epoch)?;
    let filenames_str: Vec<(&str, Detector)> =
        filenames.iter().map(|(s, d)| (s.as_str(), *d)).collect();
    calculate_fermi(&filenames_str)
}
