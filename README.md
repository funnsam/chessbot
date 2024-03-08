# My chessbot
### A chessbot on lichess
[**Go challenge me on lichess**](https://lichess.org/@/funnsams_bot) (currently accepts
non-correspondence or classical casual standard games)

## Current features
- Negamax search
- Incremental deepening
- Multithreading
- Simple evaluation function

## Quick start
1. Make a new user in lichess
2. Get an API key and register as a bot account
3. Paste API key to a new file called `.token`
4. `cargo r -r`

## Configuration
- Lichess related: `src/lichess.rs`
- Search settings: `src/bot/config.rs`
- Evaluation finetune values: `src/bot/eval.rs`
