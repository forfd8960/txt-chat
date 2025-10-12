use std::sync::Arc;

use anyhow::anyhow;
use futures::{SinkExt, StreamExt};
use tokio::io;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};
use tracing::{info, level_filters::LevelFilter, warn};
use tracing_subscriber::{Layer as _, fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt};
use txt_chat::{
    chatsvc::ChatService,
    event::{Event, handler::handle_event},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let layer = Layer::new().with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();

    let addr = "0.0.0.0:9090";
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| anyhow!("Faile to listen on {}: {}", addr, e))?;

    info!("server listen on: {}", addr);

    let chat_sevice = Arc::new(RwLock::new(ChatService::new(1000)));

    let svc = Arc::clone(&chat_sevice);

    loop {
        match listener.accept().await {
            Ok((socket, client_addr)) => {
                info!("accept conn from: {}", client_addr);

                let svc = Arc::clone(&svc);
                tokio::spawn(async move {
                    // Split the socket into read and write halves
                    let (reader, writer) = io::split(socket);

                    let mut framed_read = FramedRead::new(reader, LinesCodec::new());
                    let mut framed_write = FramedWrite::new(writer, LinesCodec::new());

                    loop {
                        match framed_read.next().await {
                            Some(frame_res) => match frame_res {
                                Ok(message) => {
                                    info!("read message from framed: {:?}", message);

                                    let event = Event::from_string(message);
                                    match event {
                                        Ok(ev) => {
                                            let svc = Arc::clone(&svc);
                                            let resp = handle_event(svc, ev).await;

                                            let _ = framed_write
                                                .send(format!("{}", resp))
                                                .await
                                                .map_err(|e| {
                                                    anyhow!("Failed to send response: {}", e)
                                                });
                                        }
                                        Err(e) => {
                                            warn!("error: {}", e);
                                        }
                                    }
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
