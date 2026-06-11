use blink_hxmt_he::io::level_1b::{SciFile, get_eng_filenames, get_sci_filenames, read_stime_offset};
use chrono::prelude::*;

/// HXMT MET epoch: 2012-01-01T00:00:00 UTC.
pub const MET_EPOCH: &str = "2012-01-01T00:00:00Z";

pub fn parse_epoch(epoch_str: &str) -> DateTime<Utc> {
    epoch_str.parse::<DateTime<Utc>>().unwrap_or_else(|_| {
        format!("{}:00:00Z", epoch_str)
            .parse::<DateTime<Utc>>()
            .expect("Invalid datetime format. Use YYYY-MM-DDTHH or full ISO 8601.")
    })
}

/// Parse a time argument that can be either MET (float) or UTC (datetime string).
pub fn parse_met_or_utc(s: &str) -> f64 {
    if let Ok(met) = s.parse::<f64>() {
        return met;
    }
    let ref_time = MET_EPOCH.parse::<DateTime<Utc>>().unwrap();
    let utc = s.parse::<DateTime<Utc>>().unwrap_or_else(|_| {
        format!("{}Z", s).parse::<DateTime<Utc>>()
            .or_else(|_| format!("{}:00Z", s).parse::<DateTime<Utc>>())
            .or_else(|_| format!("{}:00:00Z", s).parse::<DateTime<Utc>>())
            .expect("Invalid time format. Use MET number or UTC datetime (e.g. 2020-04-15T08:34:48)")
    });
    let met = (utc - ref_time).num_microseconds().unwrap() as f64 / 1e6;
    eprintln!("  UTC {} -> MET {:.6}", utc.format("%Y-%m-%dT%H:%M:%S"), met);
    met
}

/// Convert MET to its containing 1B-archive hour (floored to YYYY-MM-DDTHH:00:00 UTC).
pub fn epoch_hour_of_met(met: f64) -> DateTime<Utc> {
    let met_epoch = MET_EPOCH.parse::<DateTime<Utc>>().unwrap();
    let utc = met_epoch + chrono::Duration::microseconds((met * 1e6) as i64);
    utc.date_naive()
        .and_hms_opt(utc.hour(), 0, 0)
        .unwrap()
        .and_utc()
}

pub fn load_boxes(epoch: DateTime<Utc>) -> Vec<(String, SciFile, f64)> {
    let sci_pairs = get_sci_filenames(epoch);
    let eng_pairs = get_eng_filenames(epoch);

    sci_pairs
        .iter()
        .filter_map(|(box_name, sci_path)| {
            let sci = SciFile::new(sci_path).ok()?;
            let offset = eng_pairs
                .iter()
                .find(|(bn, _)| bn == box_name)
                .and_then(|(_, eng_path)| read_stime_offset(eng_path).ok())
                .unwrap_or(0.0);
            Some((box_name.clone(), sci, offset))
        })
        .collect()
}

pub fn filter_boxes<'a>(
    boxes: &'a [(String, SciFile, f64)],
    filter: &Option<String>,
) -> Vec<&'a (String, SciFile, f64)> {
    if let Some(fb) = filter {
        boxes.iter().filter(|(name, _, _)| name.eq_ignore_ascii_case(fb)).collect()
    } else {
        boxes.iter().collect()
    }
}

/// Warn if [met-before, met+after] crosses the hour boundary of `epoch`.
pub fn warn_if_window_crosses_hour(met: f64, before: f64, after: f64, epoch: DateTime<Utc>) {
    let met_epoch = MET_EPOCH.parse::<DateTime<Utc>>().unwrap();
    let epoch_start_met = (epoch - met_epoch).num_microseconds().unwrap() as f64 / 1e6;
    let epoch_end_met = epoch_start_met + 3600.0;
    if met - before < epoch_start_met || met + after > epoch_end_met {
        eprintln!(
            "warning: window [{:.1}, {:.1}] crosses hour boundary; only loading hour {} ({:.1}..{:.1})",
            met - before, met + after,
            epoch.format("%Y-%m-%dT%H"),
            epoch_start_met, epoch_end_met
        );
    }
}

pub fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
