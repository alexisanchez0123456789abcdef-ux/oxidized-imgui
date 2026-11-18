//! Prints the dump for one or more scenarios, so it can be diffed against the C++ reference.

use std::process::ExitCode;

use oxidized_imgui_conformance::{SCENARIOS, dump, scene};

fn main() -> ExitCode {
    let requested: Vec<String> = std::env::args().skip(1).collect();
    let names: Vec<&str> = if requested.is_empty() {
        SCENARIOS.to_vec()
    } else {
        requested.iter().map(String::as_str).collect()
    };

    for name in names {
        match scene(name) {
            Some(list) => print!("{}", dump(name, &list)),
            None => {
                eprintln!("unknown scenario: {name} (known: {})", SCENARIOS.join(", "));
                return ExitCode::from(2);
            }
        }
    }
    ExitCode::SUCCESS
}
