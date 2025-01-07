mod search;

use polars::prelude::*;
use search::calculate_fermi_nai;

fn main() {
    let filenames = [
        "current/glg_tte_n0_230101_00z_v00.fit.gz",
        "current/glg_tte_n1_230101_00z_v00.fit.gz",
        "current/glg_tte_n2_230101_00z_v00.fit.gz",
        "current/glg_tte_n3_230101_00z_v00.fit.gz",
        "current/glg_tte_n4_230101_00z_v00.fit.gz",
        "current/glg_tte_n5_230101_00z_v00.fit.gz",
        "current/glg_tte_n6_230101_00z_v00.fit.gz",
        "current/glg_tte_n7_230101_00z_v00.fit.gz",
        "current/glg_tte_n8_230101_00z_v00.fit.gz",
        "current/glg_tte_n9_230101_00z_v00.fit.gz",
        "current/glg_tte_na_230101_00z_v00.fit.gz",
        "current/glg_tte_nb_230101_00z_v00.fit.gz",
    ];
    let results = calculate_fermi_nai(&filenames);
    let df: DataFrame = df!(
        "start" => results.iter().map(|x| x.start.to_string()).collect::<Vec<_>>(),
        "stop" => results.iter().map(|x| x.stop.to_string()).collect::<Vec<_>>(),
        "bin_size_min" => results.iter().map(|x| (x.bin_size_min.total_nanoseconds() / 1000) as u32).collect::<Vec<_>>(),
        "bin_size_max" => results.iter().map(|x| (x.bin_size_max.total_nanoseconds() / 1000) as u32).collect::<Vec<_>>(),
        "bin_size_best" => results.iter().map(|x| (x.bin_size_best.total_nanoseconds() / 1000) as u32).collect::<Vec<_>>(),
        "delay" => results.iter().map(|x| (x.delay.total_nanoseconds() / 1000) as u32).collect::<Vec<_>>(),
        "count" => results.iter().map(|x| x.count).collect::<Vec<_>>(),
        "average" => results.iter().map(|x| x.average).collect::<Vec<_>>(),
    )
    .unwrap();
    println!("{}", df);
}
