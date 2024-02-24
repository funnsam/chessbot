use tokio_util::io::StreamReader;
use tokio::io::AsyncBufReadExt;
use reqwest::*;
use futures::stream::TryStreamExt;
use std::sync::{mpsc::*, Arc, Mutex};
use std::str::FromStr;
use crate::bot::*;

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

    pub async fn listen(self: Arc<Self>) {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Authorization",
            header::HeaderValue::from_str(
                &format!("Bearer {}", self.api_token)
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
                        self.games.send(game).unwrap();
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

    async fn listen_game(self: Arc<Self>, game_id: String, events: Sender<GameEvent>) {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Authorization",
            header::HeaderValue::from_str(
                &format!("Bearer {}", self.api_token)
            ).unwrap()
        );
        let client = Client::builder()
            .default_headers(headers)
            .build().unwrap();
        let stream = client.execute(
            client
                .get(format!("https://lichess.org/api/bot/game/stream/{game_id}"))
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
                    Some("gameFull") => {
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

    async fn send_game(self: Arc<Self>, game_id: String, moves: Receiver<chess::ChessMove>) {
        while let Ok(m) = moves.recv() {
            let mut m_uci = format!("{}{}", m.get_source(), m.get_dest());
            m_uci += match m.get_promotion() {
                Some(chess::Piece::Queen) => "q",
                Some(chess::Piece::Rook) => "r",
                Some(chess::Piece::Bishop) => "b",
                Some(chess::Piece::Knight) => "n",
                _ => "",
            };
        }
    }
}

#[derive(Debug)]
pub struct LichessGame {
    pub id: String,
    pub color: chess::Color,
    pub board: chess::Board,
}

pub struct GamesManager {
    incoming_games: Receiver<LichessGame>,

    games: Vec<Game>,
}

impl GamesManager {
    pub fn new(incoming_games: Receiver<LichessGame>) -> Self {
        Self { incoming_games, games: Vec::new() }
    }

    pub fn start(&mut self, client: Arc<LichessClient>) {
        loop {
            let game = self.incoming_games.recv().unwrap();

            let (event_t, event_r) = channel();

            {
                let id = game.id.clone();
                let client = Arc::clone(&client);
                tokio::spawn(async move { client.listen_game(id, event_t).await });
            }

            let (moves_t, moves_r) = channel();

            {
                let id = game.id.clone();
                let client = Arc::clone(&client);
                tokio::spawn(async move { client.send_game(id, moves_r).await });
            }

            self.games.push(Game {
                lichess: game,
                incoming_events: event_r,
                outgoing_moves: moves_t,
            });
        }
    }
}
