use std::sync::Arc;

#[macro_use]
mod log;
mod bot;
mod lichess;

#[tokio::main]
async fn main() {
    stderrlog::new().verbosity(4).init().unwrap();
    let lichess = Arc::new(lichess::LichessClient::new());
    lichess.listen().await;
}
