//! Offline fixture diagnostic; reads only stdin and emits converted JSON.
use std::io::{self, Read};
fn main() {
    let mut data = String::new();
    io::stdin().read_to_string(&mut data).unwrap();
    let inputs: Vec<String> = serde_json::from_str(&data).unwrap();
    let out: Vec<_> = inputs
        .iter()
        .map(|s| tezbar_markdown_styler::convert(s).map_err(str::to_string))
        .collect();
    println!("{}", serde_json::to_string(&out).unwrap());
}
