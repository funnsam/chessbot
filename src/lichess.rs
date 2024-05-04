use reqwest::*;
use std::sync::{atomic::*, Arc};
use std::str::FromStr;
use crate::bot::*;
use chess::*;

const DISALLOWED_TIME_CONTROLS: &[&str] = &["correspondence", "classical"];
const EXCEPTION_USERS: &[&str] = &["funnsam"];
const ACCEPT_RATED: bool = false;

pub struct LichessClient {
    client: Client,

    // FIX: see somewhere in LichessClient::send_move()
    api_token: String,

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

                    let variant = challenge["variant"]["key"].as_str().unwrap();
                    let time_ctrl = challenge["speed"].as_str().unwrap();
                    let is_rated = challenge["rated"].as_bool().unwrap();
                    if EXCEPTION_USERS.contains(&user) || (
                        variant == "standard"
                        && !DISALLOWED_TIME_CONTROLS.contains(&time_ctrl)
                        && (ACCEPT_RATED || !is_rated)
                    ) {
                        info!("`{}` challenged bot (id: `{}`)", user, id);

                        // FIX: post req
                        if client.execute(self.client
                            .post(format!("https://lichess.org/api/challenge/{id}/accept"))
                            .header("Authorization", format!("Bearer {}", self.api_token))
                            .build().unwrap()
                        ).await.ok().and_then(|a| a.status().is_success().then(|| ())).is_none() {
                            warn!("failed to accept challenge id {}", id);
                        }
                    } else {
                        if client.execute(client
                            .post(format!("https://lichess.org/api/challenge/{id}/decline"))
                            .header("Authorization", format!("Bearer {}", self.api_token))
                            .build().unwrap()
                        ).await.ok().and_then(|a| a.status().is_success().then(|| ())).is_none() {
                            warn!("failed to decline challenge id {}", id);
                        }
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

                    let game = crate::bot::Game::new(board, Vec::new());
                    let arc = Arc::clone(&self);
                    tokio::spawn(async move { arc.play_game(id, game, color).await });
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

    async fn play_game(self: Arc<Self>, game_id: String, mut game: crate::bot::Game, color: Color) {
        let color_prefix = if matches!(color, Color::White) {
            "w"
        } else {
            "b"
        };

        let stream = self.client.execute(
            self.client
                .get(format!("https://lichess.org/api/bot/game/stream/{game_id}"))
                .build().unwrap()
        ).await.unwrap().bytes_stream();
        let mut stream = NdJsonIter::new(stream);

        let mut ignore_next = false;

        while let Some(event) = stream.next_json().await {
            match event["type"].as_str() {
                Some("gameFull") => {
                    let state = &event["state"];

                    let moves = state["moves"].as_str().unwrap().split_whitespace();

                    for m in moves {
                        game.moves.push(move_from_uci(m));
                    }

                    if game.board.side_to_move() == color {
                        let time = state[color_prefix.to_string() + "time"].as_usize().unwrap();
                        let inc = state[color_prefix.to_string() + "inc"].as_usize().unwrap();

                        game.time_ctrl = TimeControl {
                            time_left: time,
                            time_incr: inc,
                        };

                        ignore_next = true;
                        let next = game.play();
                        self.send_move(&game_id, next).await;
                    }
                },
                Some("gameState") => {
                    if !ignore_next {
                        let m = event["moves"].as_str().unwrap().split_whitespace().last().unwrap();
                        let m = move_from_uci(m);
                        game.board = game.board.make_move_new(m);
                        game.moves.push(m);

                        let time = event[color_prefix.to_string() + "time"].as_usize().unwrap();
                        let inc = event[color_prefix.to_string() + "inc"].as_usize().unwrap();

                        game.time_ctrl = TimeControl {
                            time_left: time,
                            time_incr: inc,
                        };

                        ignore_next = true;
                        let next = game.play();
                        self.send_move(&game_id, next).await;
                    } else {
                        ignore_next = false;
                    }
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

    async fn send_move(&self, game_id: &str, m: ChessMove) {
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

struct NdJsonIter<S: Send + futures_util::stream::Stream<Item = Result<bytes::Bytes>>> {
    stream: S,
    buffer: Vec<u8>,
    leftover: Vec<u8>,
}

impl<S: Send + futures_util::stream::Stream<Item = Result<bytes::Bytes>> + std::marker::Unpin> NdJsonIter<S> {
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

fn move_from_uci(m: &str) -> ChessMove {
    let src = &m[0..2];
    let src = unsafe {
        Square::new(((src.as_bytes()[1] - b'1') << 3) + (src.as_bytes()[0] - b'a'))
    };

    let dst = &m[2..4];
    let dst = unsafe {
        Square::new(((dst.as_bytes()[1] - b'1') << 3) + (dst.as_bytes()[0] - b'a'))
    };

    let piece = m.as_bytes().get(4).and_then(|p| match p {
        b'n' => Some(Piece::Knight),
        b'b' => Some(Piece::Bishop),
        b'q' => Some(Piece::Queen),
        b'r' => Some(Piece::Rook),
        _ => None,
    });

    ChessMove::new(src, dst, piece)
}
