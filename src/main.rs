use futures_util::{SinkExt, StreamExt};
use std::env;

use backoff::{
    SystemClock,
    exponential::{ExponentialBackoff, ExponentialBackoffBuilder},
};
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    let host = env::var("INSTANCE_HOST")?;
    let api_token = env::var("API_TOKEN")?;

    let url = format!("ws://{}/streaming?i={}", host, api_token);

    let backoff: ExponentialBackoff<SystemClock> = ExponentialBackoffBuilder::default()
        .with_initial_interval(std::time::Duration::from_secs(1))
        .with_max_interval(std::time::Duration::from_secs(30))
        .with_multiplier(2.0)
        .with_max_elapsed_time(Some(std::time::Duration::from_secs(300)))
        .build();

    let (mut ws_stream, _) = backoff::future::retry(backoff, || {
        let url = url.clone();
        async move {
            connect_async(url.as_str())
                .await
                .map_err(backoff::Error::transient)
        }
    })
    .await
    .map_err(|e| anyhow::anyhow!("Failed to connect after retries: {}", e))?;

    let connect_message = json!({
        "type": "connect",
        "body": {
            "channel": "main",
            "id": "chordinator-test"
        }
    });

    ws_stream
        .send(Message::Text(connect_message.to_string().into()))
        .await?;

    while let Some(Ok(message)) = ws_stream.next().await {
        match message {
            Message::Text(text) => {
                println!("Received: {}", text);
            }
            Message::Binary(bin) => {
                println!("Received binary data: {:?}", bin);
            }
            Message::Ping(_) | Message::Pong(_) => {
                // Handle ping/pong if necessary
                print!("Received ping/pong message");
            }
            Message::Close(frame) => {
                println!("Connection closed: {:?}", frame);
                break;
            }
            _ => {
                println!("Received other message type: {:?}", message);
            }
        }
    }
    Ok(())
}
