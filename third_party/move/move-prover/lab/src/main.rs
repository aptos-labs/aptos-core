// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use itertools::Itertools;
use move_prover_lab::{benchmark, plot};

fn main() {
    let args = std::env::args().collect_vec();
    if args.len() < 2 || args.len() > 1 && matches!(args[1].as_str(), "-h" | "--help") {
        println!(
            "prover-lab: please specify which tool to call. Available tools: `bench`, `plot`."
        );
        println!("Use `prover-lab <tool> -h` for tool specific information.");
        std::process::exit(1);
    } else {
        match args[1].as_str() {
            "bench" => benchmark::benchmark(&args[1..]),
            "plot" => {
                if let Err(x) = plot::plot_svg(&args[1..]) {
                    println!("prover-lab: error: {}", x);
                    std::process::exit(10);
                }
            },
            _ => {
                println!("prover-lab: unknown tool `{}`", args[1]);
                std::process::exit(2);
            },
        }
    }
}
