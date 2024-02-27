use futures_util::stream::StreamExt;
#[tokio::main]
async fn main() {
    let c = reqwest::Client::new();
    let s = c.execute(c.get("http://127.0.0.1:8000").build().unwrap()).await.unwrap().bytes_stream();
    let mut s = NdJsonIter::new(s);
    while let Some(b) = s.next_json().await {
        println!("{b}");
        println!("next");
    }
}

use reqwest::*;
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

    async fn next_json(&mut self) -> Option<&str> {
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
            return std::str::from_utf8(&self.buffer).ok()
        }

        'a: while let Some(Ok(i)) = {
            dbg!("s");
            let a = self.stream.next().await;
            dbg!("e");
            a
        } {
            for (j, b) in i.iter().enumerate() {
                if *b != b'\n' {
                    self.buffer.push(*b);
                } else if !self.buffer.is_empty() {
                    self.leftover.extend(&i[j..]);
                    break 'a;
                }
            }

        }

        std::str::from_utf8(&self.buffer).ok()
    }
}
