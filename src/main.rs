//! Binary entry point; command handling lives in the library.

fn main() {
    let code = dm_database_sqllog2db::cli::run();
    if code != 0 {
        std::process::exit(code);
    }
}
