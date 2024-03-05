#![feature(inline_const)]

#[macro_use]
mod log;
mod bot;
mod lichess;

#[tokio::main]
async fn main() {
    rayon::ThreadPoolBuilder::new()
        .stack_size(16 * 1024 * 1024) // 64MiB because stack might overflow
        .thread_name(|i| format!("rayon_pool_thread_{i}"))
        .build_global().unwrap();

    lichess::LichessClient::new().start().await;
}
