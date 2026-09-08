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

The simplest way to play against it is to run the client locally (`cargo run --bin blok-rs --release`)
