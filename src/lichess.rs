use reqwest::*;
use std::sync::{mpsc::*, Arc};
use std::str::FromStr;
use crate::bot::*;

pub struct LichessClient {
    client: Client,
}

impl LichessClient {
    pub fn new() -> Self {
        let api_token = std::fs::read_to_string(".token").unwrap().trim().to_string();

        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Authorization",
            header::HeaderValue::from_str(
                &format!("Bearer {api_token}")
            ).unwrap()
        );
        let client = Client::builder()
            .default_headers(headers)
            .connection_verbose(true)
            .build().unwrap();

        Self { client }
    }

    pub async fn listen(self: Arc<Self>) {
        let stream = self.client.execute(
            self.client
                .get("https://lichess.org/api/stream/event")
                .build().unwrap()
        ).await.unwrap().bytes_stream();
        let mut stream = NdJsonIter::new(stream);

        info!("starting to listen for incoming games");

        while let Some(event) = stream.next_json().await {
            match event["type"].as_str() {
                Some("challenge") => {
                    let challenge = &event["challenge"];
                    let id = challenge["id"].as_str().unwrap();
                    let user = challenge["challenger"]["name"].as_str().unwrap();
                    if challenge["variant"]["key"] == "standard" {
                        info!("`{}` challenged bot (id: `{}`)", user, id);

                        if !self.client.execute(self.client
                            .post(format!("https://lichess.org/api/challenge/{id}/accept"))
                            .build().unwrap()
                        ).await.unwrap().status().is_success() {
                            warn!("failed to accept challenge id {}", id);
                        }
                    } else {
                        self.client.execute(self.client
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
                    let fen = game["fen"].as_str().unwrap();
                    let board = chess::Board::from_str(fen).unwrap();

                    info!("started a game with `{}` (id: `{}`, fen: `{}`)", user, id, fen);

                    let game = LichessGame { id, color, board };

                    let (event_t, event_r) = channel();

                    {
                        let id = game.id.clone();
                        let arc = Arc::clone(&self);
                        tokio::spawn(async move { arc.listen_game(id, event_t).await });
                    }

                    let (moves_t, moves_r) = channel();

                    {
                        let id = game.id.clone();
                        let arc = Arc::clone(&self);
                        tokio::spawn(async move { arc.send_game(id, moves_r).await });
                    }

                    tokio::spawn(async move {
                        Game {
                            lichess: game,
                            incoming_events: event_r,
                            outgoing_moves: moves_t,
                            trans_table: crate::bot::trans_table::TransTable::new(),
                        }.run();
                    });
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

    async fn listen_game(self: Arc<Self>, game_id: String, events: Sender<GameEvent>) {
        let stream = self.client.execute(
            self.client
                .get(format!("https://lichess.org/api/bot/game/stream/{game_id}"))
                .build().unwrap()
        ).await.unwrap().bytes_stream();
        let mut stream = NdJsonIter::new(stream);

        while let Some(event) = stream.next_json().await {
            match event["type"].as_str() {
                Some("gameFull") => {
                    let state = &event["state"];
                    events.send(GameEvent::FullGameState {
                        moves: state["moves"].as_str().unwrap().to_string(),
                        wtime: TimeControl {
                            time_left: state["wtime"].as_usize().unwrap(),
                            time_incr: state["winc"].as_usize().unwrap(),
                        },
                        btime: TimeControl {
                            time_left: state["btime"].as_usize().unwrap(),
                            time_incr: state["binc"].as_usize().unwrap(),
                        },
                        status: state["status"].as_str().unwrap().to_string(),
                    }).unwrap();
                },
                Some("gameState") => {
                    let state = &event;
                    events.send(GameEvent::NextGameState {
                        moves: state["moves"].as_str().unwrap().to_string(),
                        wtime: TimeControl {
                            time_left: state["wtime"].as_usize().unwrap(),
                            time_incr: state["winc"].as_usize().unwrap(),
                        },
                        btime: TimeControl {
                            time_left: state["btime"].as_usize().unwrap(),
                            time_incr: state["binc"].as_usize().unwrap(),
                        },
                        status: state["status"].as_str().unwrap().to_string(),
                    }).unwrap();
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

        info!("stream ended (id: `{}`)", game_id);
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

            // FIX: this fucking post request is hanging the streams
            let resp = self.client.execute(
                self.client
                    .post(format!("https://lichess.org/api/bot/game/{game_id}/move/{m_uci}"))
                    .build().unwrap()
            ).await.unwrap();

            // if !resp.status().is_success() {
            //     let reason = json::parse(&resp.text().await.unwrap()).unwrap();
            //     let reason = reason["error"].as_str().unwrap();
            //     warn!("move {} invalid ({})", m_uci, reason);
            // }
        }
    }
}

#[derive(Debug, Clone)]
pub struct LichessGame {
    pub id: String,
    pub color: chess::Color,
    pub board: chess::Board,
}

struct NdJsonIter<S: futures_util::stream::Stream<Item = Result<bytes::Bytes>>> {
    stream: S,
    buffer: Vec<u8>,
    leftover: Vec<u8>,
}

impl<S: futures_util::stream::Stream<Item = Result<bytes::Bytes>> + std::marker::Unpin> NdJsonIter<S> {
    fn new(stream: S) -> Self {
        Self {
            stream,
            buffer: Vec::new(),
            leftover: Vec::new(),
        }
    }

    async fn next_json(&mut self) -> Option<json::JsonValue> {
        self.buffer.clear();

        let mut used = 0;
        let mut done = false;
        dbg!("{:?} {}",self.leftover,self.leftover.len());
        for b in self.leftover.iter() {
            used += 1;
            if *b != b'\n' {
                self.buffer.push(*b);
            } else if !self.buffer.is_empty() {
                done = true;
                break;
            }
        }

        self.leftover = self.leftover[used..].to_vec();

        if done {
            return json::parse(std::str::from_utf8(&self.buffer).ok()?).ok();
        }

        use futures_util::stream::StreamExt;
        'a: while let Some(Ok(i)) = self.stream.next().await {
            dbg!("{:?} {}",i,i.len());
            for (j, b) in i.iter().enumerate() {
                if *b != b'\n' {
                    self.buffer.push(*b);
                } else if !self.buffer.is_empty() {
                    self.leftover.extend(&i[j..]);
                    break 'a;
                } else { std::hint::black_box(()); }
            }

        }
        json::parse(std::str::from_utf8(&self.buffer).ok()?).ok()
    }
}
