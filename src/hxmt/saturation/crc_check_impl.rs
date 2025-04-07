pub fn crc_check(data: &[u64; 8]) -> u64 {
    let crc_table = [0, 3, 6, 5, 12, 15, 10, 9, 11, 8, 13, 14, 7, 4, 1, 2];
    let mut crct = 0;
    let mut m = 1;

    // cdata is unnecessary to initialize. make the compiler happy.
    let mut cdata = 0;
    let mut cpdata;

    for j in 1..=(data.len() * 2 - 1) {
        if (j - 1) % 2 == 0 {
            cdata = data[m - 1];
            cpdata = cdata & 0b1111_0000;
            cpdata >>= 4;
            m += 1;
        } else {
            cpdata = cdata & 15;
        }
        crct = crc_table[(crct ^ cpdata) as usize];
    }

    crct
}
