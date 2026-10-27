// ForgeEdit syntax-highlighting demo
use std::collections::HashMap;

fn main() {
    let mut languages = HashMap::new();
    languages.insert("Rust", true);
    languages.insert("TypeScript", true);
    languages.insert("Python", true);
    println!("Welcome to ForgeEdit — {} supported examples", languages.len());
}
