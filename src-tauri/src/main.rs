#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(test))]
fn main() {
    if let Err(e) = eyezen::run() {
        eprintln!("Fatal: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
fn main() {}
