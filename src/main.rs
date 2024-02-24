use std::sync::Arc;

#[macro_use]
mod log;
mod bot;
mod lichess;

#[tokio::main]
async fn main() {
    let (lichess, games) = lichess::LichessClient::new();
    let lichess = Arc::new(lichess);
    {
        let lichess = Arc::clone(&lichess);
        tokio::spawn(async { lichess.listen().await });
    }

    let mut gm = lichess::GamesManager::new(games);
    gm.start(lichess);
}
