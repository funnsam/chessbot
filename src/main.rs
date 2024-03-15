#![warn(
    clippy::complexity,
    clippy::correctness,
    clippy::perf,
    clippy::nursery,
    clippy::suspicious,
    clippy::style,
)]
#![allow(
    clippy::semicolon_inside_block,
    clippy::just_underscores_and_digits,
)]

#![feature(new_uninit)]

#[macro_use]
mod log;
mod bot;

#[cfg(feature = "lichess")]
mod lichess;

#[cfg(feature = "uci")]
mod uci;

#[tokio::main]
async fn main() {
    rayon::ThreadPoolBuilder::new()
        .stack_size(16 * 1024 * 1024)
        .thread_name(|i| format!("rayon_pool_thread_{i}"))
        .build_global().unwrap();

    #[cfg(feature = "lichess")]
    lichess::LichessClient::new().start().await;

    #[cfg(feature = "uci")]
    uci::UciClient::new().start().await;
}

#[cfg(all(feature = "lichess", feature = "uci"))]
compile_error!("conflicting features");
