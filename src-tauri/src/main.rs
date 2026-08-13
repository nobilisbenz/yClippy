// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `list` answers on stdout and exits without ever starting the GUI, so
    // pickers in Neovim and the TUI can shell out to it.
    let args: Vec<String> = std::env::args().collect();
    if let Some(yclippy_lib::play::Command::List { query, limit }) =
        yclippy_lib::play::parse_play_args(&args)
    {
        match yclippy_lib::play::run_list(query, limit) {
            Ok(json) => {
                println!("{json}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("yclippy list: {e}");
                std::process::exit(1);
            }
        }
    }

    yclippy_lib::run()
}
