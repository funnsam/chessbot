mod lichess;

fn main() {
    let mut lichess = lichess::LichessClient::new();
    lichess.listen();
}
