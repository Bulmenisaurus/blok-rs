# blok_rs

## About

This is a rust implementation of a minimax blokus engine in rust.

## Commands

- Client: `cargo run --bin blok-rs --release`
- Test: `cargo test --release`
- Search test (samply, 2s/move until end): `samply record cargo run --bin search_test --release --features until-end`
- Search test (time, 1M nodes): `time cargo run --bin search_test --release --features nodes-1m`
- Search test (hyperfine, 10k nodes): `hyperfine --warmup 1 --runs 20 'cargo run --release --bin search_test --features nodes-10k'`

## How to play?

The simplest way to play against it is to run the client locally (`cargo run --bin blok-rs --release`). Then to go https://bulmenisaurus.github.io/blok and connect with the `Local` checkbox. On placing a move, you should be able to see thinking traces in the console.

Example:

```
Received: {"type":"findMove","move":18840}
Client message: FindMove { move: Some(18840) }
depth 1 bestmove 80832 score 174 nodes 421
depth 2 bestmove 80832 score -775 nodes 2348
depth 3 bestmove 80832 score 283 nodes 2852
depth 4 bestmove 80832 score -608 nodes 6198
depth 5 bestmove 84922 score 141 nodes 34393
depth 6 bestmove 80952 score -589 nodes 114998
```
