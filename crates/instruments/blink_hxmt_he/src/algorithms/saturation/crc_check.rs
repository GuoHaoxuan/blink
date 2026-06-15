/// CRC-4 校验，使用 CRC-4/ITU 查找表。
/// 对 8 字节数据计算 CRC-4，覆盖前 7 字节全部 + 第 8 字节高 4 位。
pub fn crc_check(data: &[u64; 8]) -> u64 {
    const CRC_TABLE: [u64; 16] = [0, 3, 6, 5, 12, 15, 10, 9, 11, 8, 13, 14, 7, 4, 1, 2];

    let mut crc: u64 = 0;
    let mut m = 0usize;
    let mut cdata: u64 = 0;

    for j in 1..=15 {
        let nibble = if (j - 1) % 2 == 0 {
            cdata = data[m];
            m += 1;
            (cdata & 0xF0) >> 4
        } else {
            cdata & 0xF
        };
        crc = CRC_TABLE[(crc ^ nibble) as usize];
    }

    crc
}
