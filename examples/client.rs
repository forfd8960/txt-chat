use anyhow::Result;
use futures::{SinkExt, StreamExt};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpStream;
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};
use tracing::{info, level_filters::LevelFilter, warn};
use tracing_subscriber::{Layer as _, fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    let layer = Layer::new().with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();

    // Connect to the server
    let stream = TcpStream::connect("0.0.0.0:9090").await?;
    info!("Connected to 0.0.0.0:9090");

    // Split the stream into read and write halves
    let (read_half, write_half) = stream.into_split();

    // Create framed reader and writer with LinesCodec
    let mut framed_read = FramedRead::new(read_half, LinesCodec::new());
    let mut framed_write = FramedWrite::new(write_half, LinesCodec::new());

    // Spawn a task to handle sending lines from stdin
    let send_task = tokio::spawn(async move {
        let stdin = BufReader::new(tokio::io::stdin());
        let mut lines = stdin.lines();

        loop {
            let line_res = lines.next_line().await;
            match line_res {
                Ok(Some(line)) => {
                    if framed_write.send(line).await.is_err() {
                        warn!("Failed to send line");
                        break;
                    }
                }
                Ok(None) | Err(_) => break,
            }
        }

        // Gracefully shut down the write half
        if let Err(e) = <FramedWrite<OwnedWriteHalf, LinesCodec> as SinkExt<String>>::close(&mut framed_write).await {
            warn!("Failed to close writer: {}", e);
        }
    });

    // Read lines from the server until EOF or error
    while let Some(line_result) = framed_read.next().await {
        match line_result {
            Ok(line) => {
                println!(">> {}", line);
            }
            Err(e) => {
                eprintln!("Error reading line: {}", e);
                break;
            }
        }
    }

    // Wait for the send task to complete
    if let Err(e) = send_task.await {
        eprintln!("Send task failed: {}", e);
    }

    println!("Connection closed.");
    Ok(())
}
