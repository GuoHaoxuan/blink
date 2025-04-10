use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::{LazyLock, Mutex};

static CACHED_ACD: LazyLock<Mutex<Option<Vec<Vec<f64>>>>> = LazyLock::new(|| Mutex::new(None));

fn get_acd_data() -> Vec<Vec<f64>> {
    let mut cached_acd = CACHED_ACD.lock().unwrap();

    if let Some(ref acd) = *cached_acd {
        return acd.clone();
    }

    // 读取文件
    // Get the directory of the current file
    let current_file = std::path::Path::new(file!());
    let parent_dir = current_file
        .parent()
        .expect("Failed to get parent directory");
    let acd_path = parent_dir.join("acd.txt");
    let file = File::open(&acd_path).expect("无法打开ACD文件");
    let reader = BufReader::new(file);

    // 解析文件内容到向量
    let mut data_vec: Vec<Vec<f64>> = Vec::new();

    for line in reader.lines() {
        let line = line.expect("读取行失败");
        let values: Vec<f64> = line
            .split_whitespace()
            .filter_map(|s| s.parse::<f64>().ok())
            .collect();

        if !values.is_empty() {
            data_vec.push(values);
        }
    }

    *cached_acd = Some(data_vec.clone());
    data_vec
}

pub(crate) fn interpolate_point(lon: f64, lat: f64) -> f64 {
    let acd = get_acd_data();

    // 提取子数组
    let acdx: Vec<Vec<f64>> = acd[0..180].to_vec();
    let acdy: Vec<Vec<f64>> = acd[180..360].to_vec();
    let data1: Vec<Vec<f64>> = acd[360..].to_vec();

    // 替换NaN值为0
    let mut data = data1.clone();
    for row in &mut data {
        for elem in row.iter_mut() {
            if elem.is_nan() {
                *elem = 0.0;
            }
        }
    }

    // 找到插值的索引
    let acdx_row = &acdx[0];
    let mut lon_index = acdx_row
        .iter()
        .position(|&x| x > lon)
        .unwrap_or(acdx_row.len() - 1);
    if lon_index == 0 {
        lon_index = 1; // 确保我们有前一个点
    }

    let mut acdy_col = Vec::new();
    for row in &acdy {
        acdy_col.push(row[0]);
    }

    let mut lat_index = acdy_col
        .iter()
        .position(|&y| y > lat)
        .unwrap_or(acdy_col.len() - 1);
    if lat_index == 0 {
        lat_index = 1; // 确保我们有前一个点
    }

    // 计算插值的分数
    let lon_fraction =
        (lon - acdx_row[lon_index - 1]) / (acdx_row[lon_index] - acdx_row[lon_index - 1]);
    let lat_fraction =
        (lat - acdy_col[lat_index - 1]) / (acdy_col[lat_index] - acdy_col[lat_index - 1]);

    // 进行双线性插值计算
    let interpolated_value =
        (1.0 - lon_fraction) * (1.0 - lat_fraction) * data[lat_index - 1][lon_index - 1]
            + lon_fraction * (1.0 - lat_fraction) * data[lat_index - 1][lon_index]
            + (1.0 - lon_fraction) * lat_fraction * data[lat_index][lon_index - 1]
            + lon_fraction * lat_fraction * data[lat_index][lon_index];

    interpolated_value
}
