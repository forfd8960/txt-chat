use std::sync::Arc;

use anyhow::anyhow;
use futures::{SinkExt, StreamExt};
use tokio::io::{self, WriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::Sender;
use tokio::sync::{RwLock, broadcast};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};
use tracing::{info, level_filters::LevelFilter, warn};
use tracing_subscriber::{Layer as _, fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt};
use txt_chat::chatsvc::Message;
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

    let (tx, _) = broadcast::channel::<Message>(1000);
    let tx1 = tx.clone();
    let chat_sevice = Arc::new(RwLock::new(ChatService::new(1000, tx)));

    loop {
        match listener.accept().await {
            Ok((socket, client_addr)) => {
                info!("accept conn from: {}", client_addr.clone());

                let uid = format!("{:?}", client_addr);
                let tx_clone = tx1.clone();
                let svc = chat_sevice.clone();
                tokio::spawn(async move {
                    serve_conn(socket, uid, tx_clone, &svc).await;
                });
            }
            Err(e) => warn!("Faield to accept conn: {}", e),
        }
    }
}

async fn serve_conn(
    socket: TcpStream,
    uid: String,
    tx: Sender<Message>,
    chat_sevice: &Arc<RwLock<ChatService>>,
) {
    // Split the socket into read and write halves
    let (reader, writer) = io::split(socket);

    let mut framed_read = FramedRead::new(reader, LinesCodec::new());
    let mut framed_write = FramedWrite::new(writer, LinesCodec::new());

    let user_id1 = uid.clone();
    let mut rx = tx.subscribe();

    let svc1 = chat_sevice.clone();
    let svc2 = chat_sevice.clone();

    tokio::spawn(async move {
        info!("recv msg for user: {}", user_id1);
        recv_msg(svc1, user_id1, &mut rx, &mut framed_write).await;
    });

    loop {
        match framed_read.next().await {
            Some(frame_res) => match frame_res {
                Ok(message) => {
                    info!("read message from client: {:?}", message);

                    let event = Event::from_string(message);
                    match event {
                        Ok(ev) => {
                            info!("handle event: {:?}", ev);
                            handle_event(uid.clone(), svc2.clone(), ev).await;
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
}

async fn recv_msg(
    svc: Arc<RwLock<ChatService>>,
    uid: String,
    rx: &mut broadcast::Receiver<Message>,
    framed_write: &mut FramedWrite<WriteHalf<TcpStream>, LinesCodec>,
) {
    let _ = framed_write
        .send("------Welcome to Txt Chat------".to_string())
        .await
        .map_err(|e| anyhow!("Failed to send response: {}", e));

    while let Ok(msg) = rx.recv().await {
        let svc = svc.read().await;
        if !svc.is_user_sub(&uid, &msg.chan_id) {
            drop(svc);
            continue;
        }
        drop(svc);

        let _ = framed_write
            .send(msg.to_string())
            .await
            .map_err(|e| anyhow!("Failed to send response: {}", e));
    }
}
