mod search;

use itertools::Itertools;
use regex::Regex;
use std::str::FromStr;

use hifitime::prelude::*;
use polars::prelude::*;
use search::calculate_fermi_nai;

fn get_fermi_nai_filenames(epoch: Epoch) -> Vec<String> {
    let (y, m, d, h, ..) = epoch.to_gregorian_utc();
    let folder = format!(
        "/gecamfs/Exchange/GSDC/missions/FTP/fermi/data/gbm/daily/{:04}/{:02}/{:02}/current",
        y, m, d
    );
    (0..12)
        .map(|i| {
            format!(
                "glg_tte_n{:x}_{:02}{:02}{:02}_{:02}z_v\\d{{2}}\\.fit\\.gz",
                i,
                y % 100,
                m,
                d,
                h,
            )
        })
        .inspect(|x| println!("{}", x))
        .map(|x| Regex::new(&x).unwrap())
        .map(|re| {
            std::fs::read_dir(&folder)
                .unwrap()
                .map(|x| x.unwrap())
                .map(|x| x.path())
                .filter(|x| x.is_file())
                .map(|x| x.file_name().unwrap().to_str().unwrap().to_string())
                .filter(|x| re.is_match(x))
                .max_by(|a, b| {
                    let extract = |x: &str| {
                        x.split('_')
                            .last()
                            .unwrap()
                            .strip_prefix('v')
                            .unwrap()
                            .strip_suffix(".fit.gz")
                            .unwrap()
                            .parse::<u32>()
                            .unwrap()
                    };
                    extract(a).cmp(&extract(b))
                })
                .unwrap()
        })
        .collect::<Vec<_>>()
}
fn main() {
    // let filenames = [
    //     "current/glg_tte_n0_230101_00z_v00.fit.gz",
    //     "current/glg_tte_n1_230101_00z_v00.fit.gz",
    //     "current/glg_tte_n2_230101_00z_v00.fit.gz",
    //     "current/glg_tte_n3_230101_00z_v00.fit.gz",
    //     "current/glg_tte_n4_230101_00z_v00.fit.gz",
    //     "current/glg_tte_n5_230101_00z_v00.fit.gz",
    //     "current/glg_tte_n6_230101_00z_v00.fit.gz",
    //     "current/glg_tte_n7_230101_00z_v00.fit.gz",
    //     "current/glg_tte_n8_230101_00z_v00.fit.gz",
    //     "current/glg_tte_n9_230101_00z_v00.fit.gz",
    //     "current/glg_tte_na_230101_00z_v00.fit.gz",
    //     "current/glg_tte_nb_230101_00z_v00.fit.gz",
    // ];
    // let results = calculate_fermi_nai(&filenames);
    // let df: DataFrame = df!(
    //     "start" => results.iter().map(|x| x.start.to_string()).collect::<Vec<_>>(),
    //     "stop" => results.iter().map(|x| x.stop.to_string()).collect::<Vec<_>>(),
    //     "bin_size_min" => results.iter().map(|x| (x.bin_size_min.total_nanoseconds() / 1000) as u32).collect::<Vec<_>>(),
    //     "bin_size_max" => results.iter().map(|x| (x.bin_size_max.total_nanoseconds() / 1000) as u32).collect::<Vec<_>>(),
    //     "bin_size_best" => results.iter().map(|x| (x.bin_size_best.total_nanoseconds() / 1000) as u32).collect::<Vec<_>>(),
    //     "delay" => results.iter().map(|x| (x.delay.total_nanoseconds() / 1000) as u32).collect::<Vec<_>>(),
    //     "count" => results.iter().map(|x| x.count).collect::<Vec<_>>(),
    //     "average" => results.iter().map(|x| x.average).collect::<Vec<_>>(),
    // )
    // .unwrap();
    // println!("{}", df);
    let filenames = get_fermi_nai_filenames(Epoch::from_str("2023-01-01T00:00:00").unwrap());
    println!("{:?}", filenames);
}
