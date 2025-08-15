use std::env;
use ft_ality_parse::{parse_gmr_file};

fn main() {
    let path = env::args().nth(1).expect("usage: ft_ality <file.gmr>");
    let debug: bool = env::args().any(|arg| arg == "--debug" || arg == "-d");
    let grammar = parse_gmr_file(&path).expect("failed to parse file");


    if debug {
        println!("Key mappings (inferred from Σ):");
        for (i, tok) in grammar.alphabet.iter().enumerate() {
            println!("{} -> {}", i, tok.as_str());
        }
        println!("----------------------\n");

        for rule in &grammar.rules {
            let seq = rule
                .sequence
                .iter()
                .map(|t| t.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            println!("{seq}  =>  {}", rule.move_name);
        }
    }
}
