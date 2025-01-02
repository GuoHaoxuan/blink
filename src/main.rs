mod search;

fn main() {
    let results = search::calculate_hxmt("HXMT_20190205T11_HE-Evt_FFFFFF_V1_1K.FITS");
    println!("Lenth of results: {}", results.len());
    println!("{:#?}", results);
}
