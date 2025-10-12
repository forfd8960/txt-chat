use anyhow::Result;
use tokio::{io, net::TcpListener};

use anyhow::anyhow;
use futures::{SinkExt, StreamExt};
use tokio_util::codec::LinesCodec;
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::{info, level_filters::LevelFilter, warn};
use tracing_subscriber::{Layer as _, fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    let layer = Layer::new().with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();

    let addr = "0.0.0.0:9090";
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| anyhow!("Faile to listen on {}: {}", addr, e))?;

    info!("server listen on: {}", addr);

    loop {
        match listener.accept().await {
            Ok((socket, client_addr)) => {
                info!("accept conn from: {}", client_addr);

                tokio::spawn(async move {
                    // Split the socket into read and write halves
                    let (reader, writer) = io::split(socket);

                    // Create framed reader and writer with Resp2Codec
                    let mut framed_read = FramedRead::new(reader, LinesCodec::new());

                    let mut framed_write = FramedWrite::new(writer, LinesCodec::new());

                    loop {
                        match framed_read.next().await {
                            Some(frame_res) => match frame_res {
                                Ok(message) => {
                                    info!("read message from framed: {:?}", message);
                                    let _ = framed_write
                                        .send(format!("echo: {}", message))
                                        .await
                                        .map_err(|e| anyhow!("Failed to send response: {}", e));
                                }
                                Err(e) => {
                                    warn!("fail read frame: {:?}", e);
                                    break;
                                }
                            },
                            None => {
                                warn!("No frame");
                                break;
                            }
                        }
                    }
                });
            }
            Err(e) => warn!("Faield to accept conn: {}", e),
        }
    }
}
