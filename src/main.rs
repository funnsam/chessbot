#![feature(inline_const)]

#[macro_use]
mod log;
mod bot;
mod lichess;

#[tokio::main]
async fn main() {
    lichess::LichessClient::new().start().await;
}
