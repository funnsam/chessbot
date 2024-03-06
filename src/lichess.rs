use reqwest::*;
use std::sync::{atomic::*, mpsc::*, Arc};
use std::str::FromStr;
use crate::bot::*;

pub struct LichessClient {
    client: Client,

    // FIX: see somewhere in LichessClient::send_game()
    api_token: String,

    pub enable_pair: AtomicBool,
    pub active_games: AtomicUsize,
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

        Self {
            client,
            api_token,

            enable_pair: AtomicBool::new(false),
            active_games: AtomicUsize::new(0),
        }
    }

    pub async fn start(self) {
        let li = Arc::new(self);

        {
            let li = Arc::clone(&li);
            tokio::spawn(async { li.auto_challenge().await });
        }

        li.listen().await;
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

                    let client = Client::new();
                    let is_me = user == "funnsam";
                    if (challenge["variant"]["key"] == "standard"
                        && challenge["speed"] != "correspondence"
                        && !challenge["rated"].as_bool().unwrap()) || is_me {
                        info!("`{}` challenged bot (id: `{}`)", user, id);

                        // FIX: post req
                        if !client.execute(self.client
                            .post(format!("https://lichess.org/api/challenge/{id}/accept"))
                            .header("Authorization", format!("Bearer {}", self.api_token))
                            .build().unwrap()
                        ).await.unwrap().status().is_success() {
                            warn!("failed to accept challenge id {}", id);
                        }
                    } else {
                        client.execute(client
                            .post(format!("https://lichess.org/api/challenge/{id}/decline"))
                            .header("Authorization", format!("Bearer {}", self.api_token))
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
                            outgoing_moves: moves_t,
                            trans_table: crate::bot::trans_table::TransTable::new(),
                            age: 1,

                            time_ctrl: TimeControl::default(),
                            time_ref: std::time::Instant::now(),
                            time_usable: std::time::Duration::from_secs(0),
                        }.run(event_r);
                    });
                },
                Some("gameFinish") => {
                    self.active_games.fetch_add(1, Ordering::Relaxed);
                },
                Some("challengeCanceled" | "challengeDeclined") => {},
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
            // FIX: this fucking post request is hanging the streams
            // https://github.com/seanmonstar/reqwest/issues/2133
            //
            // let resp = self.client.execute(
            //     self.client
            //         .post(format!("https://lichess.org/api/bot/game/{game_id}/move/{m_uci}"))
            //         .build().unwrap()
            // ).await.unwrap();

            let client = Client::new();
            let resp = client.execute(
                client
                    .post(format!("https://lichess.org/api/bot/game/{game_id}/move/{m}"))
                    .header("Authorization", format!("Bearer {}", self.api_token))
                    .build().unwrap()
            ).await.unwrap();

            if !resp.status().is_success() {
                let reason = json::parse(&resp.text().await.unwrap()).unwrap();
                let reason = reason["error"].as_str().unwrap();
                warn!("move {} invalid ({})", m, reason);
            }
        }
    }

    pub async fn auto_challenge(self: Arc<Self>) {
        // let client = Client::new();
        // loop {
        //     tokio::time::sleep(std::time::Duration::from_secs(15)).await;

        //     if self.active_games.load(Ordering::Relaxed) < super::config::NUM_ACTIVE_GAMES {
        //         let resp = client.execute(
        //             client
        //                 .post(format!("https://lichess.org/api/challenge/{target}"))
        //                 .header("Authorization", format!("Bearer {}", self.api_token))
        //                 .build().unwrap()
        //         ).await.unwrap();
        //     }
        // }
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

pub enum ServerCommand {
    AutoChallenge {
        enable: bool,
    },
}
