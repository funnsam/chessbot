use tokio_util::io::StreamReader;
use tokio::io::AsyncBufReadExt;
use std::sync::mpsc::*;
use reqwest::*;
use futures::stream::TryStreamExt;
use std::sync::Mutex;
use std::str::FromStr;

pub struct LichessClient {
    api_token: String,
    games: Sender<LichessGame>,
}

impl LichessClient {
    pub fn new() -> (Self, Receiver<LichessGame>) {
        let (games, games_r) = channel();
        (Self {
            api_token: std::fs::read_to_string(".token").unwrap().trim().to_string(),
            games,
        }, games_r)
    }

    pub async fn listen(this: Mutex<Self>) {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Authorization",
            header::HeaderValue::from_str(
                &format!("Bearer {}", this.lock().unwrap().api_token)
            ).unwrap()
        );
        let client = Client::builder()
            .default_headers(headers)
            .build().unwrap();
        let stream = client.execute(
            client
                .get("https://lichess.org/api/stream/event")
                .build().unwrap()
            )
            .await.unwrap().bytes_stream();
        let mut stream = StreamReader::new(
            stream.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        ).lines();

        while let Ok(Some(event)) = stream.next_line().await {
            if !event.is_empty() {
                let event = json::parse(&event).unwrap();

                match event["type"].as_str() {
                    Some("challenge") => {
                        let challenge = &event["challenge"];
                        let id = challenge["id"].as_str().unwrap();
                        let user = challenge["challenger"]["name"].as_str().unwrap();
                        if challenge["variant"]["key"] == "standard" {
                            info!("`{}` challenged bot (id: `{}`)", user, id);

                            if !client.execute(client
                                .post(format!("https://lichess.org/api/challenge/{id}/accept"))
                                .build().unwrap()
                            ).await.unwrap().status().is_success() {
                                warn!("failed to accept challenge id {}", id);
                            }
                        } else {
                            client.execute(client
                                .post(format!("https://lichess.org/api/challenge/{id}/decline"))
                                .build().unwrap()
                            ).await.unwrap();
                        }
                    },
                    Some("gameStart") => {
                        let game = &event["game"];
                        let id = game["id"].as_str().unwrap().to_string();
                        let user = game["opponent"]["username"].as_str().unwrap();
                        let color = match game["color"].as_str() {
                            Some("black") => chess::Color::Black,
                            Some("white") => chess::Color::White,
                            v => {
                                warn!("unknown color `{:?}`", v);
                                continue;
                            },
                        };
                        let board = chess::Board::from_str(game["fen"].as_str().unwrap()).unwrap();

                        info!("started a game with `{}` (id: `{}`)", user, id);

                        let game = LichessGame { id, color, board };
                        dbg!("{:?}", game);
                        this.lock().unwrap().games.send(game).unwrap();
                    },
                    Some(typ) => {
                        warn!("got unknown type of event `{}`", typ);
                        dbg!("{:?}", event);
                    },
                    None => {
                        warn!("got unknown type of event");
                        dbg!("{:?}", event);
                    },
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct LichessGame {
    id: String,
    pub color: chess::Color,
    pub board: chess::Board,
}

pub struct GamesManager {
    incoming_games: Receiver<LichessGame>,

    games: Vec<crate::bot::Game>,
}

impl GamesManager {
    pub fn new(incoming_games: Receiver<LichessGame>) -> Self {
        Self { incoming_games, games: Vec::new() }
    }

    pub fn start(&mut self) {
        loop {
            let game = self.incoming_games.recv().unwrap();

            // TODO:
        }
    }
}
