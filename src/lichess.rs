use std::sync::mpsc::*;
use reqwest::*;

pub struct LichessClient {
    api_token: String,
    challenges: Sender<LichessChallenge>,
    pub challenges_r: Receiver<LichessChallenge>,
}

impl LichessClient {
    pub fn new() -> Self {
        let (challenges, challenges_r) = channel();
        Self {
            api_token: std::fs::read_to_string(".token").unwrap(),
            challenges,
            challenges_r,
        }
    }

    pub async fn listen(&mut self) {
        let mut stream = get("https://lichess.org/api/stream/event").await.unwrap().bytes_stream();
    }
}

pub struct LichessChallenge {
    id: String,
}
