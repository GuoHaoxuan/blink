mod search;

fn main() {
    let results = search::calculate("xxxxxxxx");
    println!("Lenth of results: {}", results.len());
    println!("{:#?}", results);
}
