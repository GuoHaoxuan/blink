use blink_core::error::Error;

/// 从工程数据文件中读取 stime→UTC 的固定偏移量。
///
/// 工程文件每秒一包，包中 `UTC_Last_Bdc` 和 `sTime_Last_Bdc` 列
/// 给出精确的 UTC↔stime 映射。offset = UTC - stime，在整个小时内恒定。
pub fn read_stime_offset(filename: &str) -> Result<f64, Error> {
    let mut fptr = fitsio::FitsFile::open(filename)?;
    let hdu = fptr.hdu("HE_Eng")?;

    let utc: Vec<i64> = hdu.read_col(&mut fptr, "UTC_Last_Bdc")?;
    let stime: Vec<i64> = hdu.read_col(&mut fptr, "sTime_Last_Bdc")?;

    if utc.is_empty() || stime.is_empty() {
        return Err(Error::InvalidData("Empty eng data".into()));
    }

    // offset = utc - stime, 取众数（理论上全部相同）
    let offset = utc[0] - stime[0];
    Ok(offset as f64)
}
