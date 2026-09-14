//! Binary entry point; command handling lives in the library.
// Log parsing creates many short-lived strings; jemalloc reduces allocator overhead.
#[cfg(not(target_os = "windows"))]
#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[tokio::main]
async fn main() {
    let code = dm_database_sqllog2db::cli::run().await;
    if code != 0 {
        std::process::exit(code);
    }
}
