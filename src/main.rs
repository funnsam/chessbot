use tokio::spawn;
use std::sync::Mutex;

#[macro_use]
mod log;
mod bot;
mod lichess;

#[tokio::main]
async fn main() {
    let (lichess, games) = lichess::LichessClient::new();
    let lichess = Mutex::new(lichess);
    tokio::spawn(async { lichess::LichessClient::listen(lichess).await });

    let mut gm = lichess::GamesManager::new(games);
    gm.start();
}
