mod board;
mod mcts;
mod movegen;
mod nn;

use futures_util::{SinkExt, StreamExt};
use mcts::MonteCarlo;
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{WebSocketStream, accept_async};

use crate::nn::NNUE;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum ServerMessage {
    #[serde(rename = "init")]
    Init {
        #[serde(rename = "startPos")]
        start_pos: String,
        difficulty: String,
    },
    #[serde(rename = "findMove")]
    FindMove { r#move: Option<u32> },
}

async fn handle_websocket(ws_stream: WebSocketStream<TcpStream>) {
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let mut board = board::BoardState::new(board::StartPosition::Corner, NNUE);
    let mut eval: MonteCarlo = MonteCarlo::new(NNUE, false);
    let mut game_difficulty: String = "hard".to_string();

    // Handle incoming messages
    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                println!("Received: {}", text);

                // Parse JSON message
                match serde_json::from_str::<ServerMessage>(&text) {
                    Ok(client_msg) => {
                        println!("Client message: {:?}", client_msg);
                        match client_msg {
                            ServerMessage::Init {
                                start_pos,
                                difficulty,
                            } => {
                                println!("Client requested init");
                                let start_position = match start_pos.as_str() {
                                    "middle" => board::StartPosition::Middle,
                                    "corner" => board::StartPosition::Corner,
                                    "middleBlokee" => board::StartPosition::MiddleBlokee,
                                    _ => board::StartPosition::Middle,
                                };
                                board.start_position = start_position;
                                game_difficulty = difficulty;
                            }
                            ServerMessage::FindMove { r#move } => {
                                if let Some(last_move) = r#move {
                                    board.do_move(last_move);
                                }

                                eval.run_search(&board, &game_difficulty);
                                let best_move = eval.best_play().unwrap();
                                board.do_move(best_move);
                                eval.clear();

                                let response_json =
                                    format!("{{\"type\": \"move\", \"move\": {}}}", best_move);

                                if let Err(e) = ws_sender.send(Message::Text(response_json)).await {
                                    eprintln!("Failed to send move response: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to parse JSON: {}", e);
                    }
                }
            }
            Ok(Message::Close(_)) => {
                println!("Client disconnected");
                break;
            }
            Ok(m) => {
                println!("Received unknown message: {:?}", m);
                // Ignore other message types
            }
            Err(e) => {
                eprintln!("WebSocket error: {}", e);
                break;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    // Start WebSocket server
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await.expect("Failed to bind");
    println!("WebSocket server listening on ws://{}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let ws_stream = accept_async(stream)
            .await
            .expect("Failed to accept WebSocket");

        tokio::spawn(async move {
            handle_websocket(ws_stream).await;
        });
    }
}
