use anyhow::Result;
use futures::{SinkExt, StreamExt};
use std::collections::HashSet;
use std::env;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::RwLock;
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};
use tracing::{info, level_filters::LevelFilter, warn};
use tracing_subscriber::{Layer as _, fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt};
use txt_chat::errors::ChatErrors;

const JOIN: &'static str = "$join";
const JOIN_RESP: &'static str = "$$joined";

pub struct ClientState {
    pub user_id: String,
    pub user_chan: String,    // automic created chan after user register
    pub current_chan: String, // current chan id
    pub joined_chans: HashSet<String>,
}

impl ClientState {
    pub fn new(user_id: String) -> Self {
        Self {
            user_id,
            user_chan: "".to_string(),
            current_chan: "".to_string(),
            joined_chans: HashSet::new(),
        }
    }

    pub fn set_uid(&mut self, uid: String) {
        self.user_id = uid;
    }

    pub fn append_chan(&mut self, chan_id: String) {
        self.joined_chans.insert(chan_id);
    }

    pub fn leave_chan(&mut self, chan_id: String) {
        self.joined_chans.remove(&chan_id);
        if self.current_chan == chan_id {
            self.current_chan = self.user_chan.clone();
        }
    }

    pub fn switch_chan(&mut self, new_chan: String) {
        self.current_chan = new_chan.clone();
        self.append_chan(new_chan.clone());
        info!("switched to channel: {}", new_chan);
    }
}

fn encode_reg(uname: String) -> String {
    format!("reg${}", uname)
}

// join$123$456
fn encode_join(state: &ClientState, chan_id: String) -> String {
    format!("join${}${}", state.user_id, chan_id)
}

// leave$123$456
fn encode_leave(state: &ClientState, chan_id: String) -> String {
    format!("leave${}${}", state.user_id, chan_id)
}

// create_chan$123$MyChat
fn encode_create_chan(state: &ClientState, chan_name: String) -> String {
    format!("create_chan${}${}", state.user_id, chan_name)
}

// send_msg${uid}${chan_id}$Hello
fn encode_send_msg(state: &ClientState, msg: String) -> Result<String, ChatErrors> {
    if state.current_chan.is_empty() {
        return Err(ChatErrors::UnknownCurrentChan);
    }

    Ok(format!(
        "send_msg${}${}${}",
        state.user_id, state.current_chan, msg
    ))
}

#[tokio::main]
async fn main() -> Result<()> {
    let layer = Layer::new().with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();

    // Connect to the server
    let stream = TcpStream::connect("0.0.0.0:9090").await?;
    info!("Connected to 0.0.0.0:9090");

    let user_id = format!("{:?}", stream.local_addr()?);

    // Split the stream into read and write halves
    let (read_half, write_half) = stream.into_split();

    // Create framed reader and writer with LinesCodec
    let mut framed_read = FramedRead::new(read_half, LinesCodec::new());
    let mut framed_write = FramedWrite::new(write_half, LinesCodec::new());

    let args: Vec<String> = env::args().collect();
    let user_name = &args[1];

    match framed_write.send(encode_reg(user_name.to_string())).await {
        Ok(_) => {}
        Err(e) => {
            warn!("register user failed: {}", e);
            return Ok(());
        }
    }

    let state = Arc::new(RwLock::new(ClientState::new(user_id)));
    if let Some(Ok(cur_chan)) = framed_read.next().await {
        let mut state = state.write().await;
        state.current_chan = cur_chan
    } else {
        warn!("register user failed");
        return Ok(());
    }
    let state_clone = state.clone();
    let state_clone1 = state.clone();

    // Spawn a task to handle sending lines from stdin
    let send_task = tokio::spawn(async move {
        let stdin = BufReader::new(tokio::io::stdin());
        let mut lines = stdin.lines();

        loop {
            let line_res = lines.next_line().await;
            match line_res {
                Ok(Some(line)) => {
                    if line.trim().is_empty() {
                        continue;
                    }

                    let state = state_clone.read().await;
                    let (yes, chan_id) = match is_join(line.clone()) {
                        Ok((yes, chan_id)) => (yes, chan_id),
                        Err(e) => {
                            warn!("error: {}", e);
                            (false, "".to_string())
                        }
                    };

                    if yes {
                        let msg = encode_join(&state, chan_id);
                        if framed_write.send(msg).await.is_err() {
                            warn!("Failed to send line");
                            break;
                        }
                    } else {
                        if let Ok(msg) = encode_send_msg(&state, line) {
                            if framed_write.send(msg).await.is_err() {
                                warn!("Failed to send line");
                                break;
                            }
                        }
                    }
                }
                Ok(None) | Err(_) => break,
            }
        }

        // Gracefully shut down the write half
        if let Err(e) =
            <FramedWrite<OwnedWriteHalf, LinesCodec> as SinkExt<String>>::close(&mut framed_write)
                .await
        {
            warn!("Failed to close writer: {}", e);
        }
    });

    // Read lines from the server until EOF or error
    while let Some(line_result) = framed_read.next().await {
        match line_result {
            Ok(line) => {
                println!(">> {}", line);

                let mut state = state_clone1.write().await;
                if let Ok(joined_chan) = parse_join_resp(&line) {
                    state.switch_chan(joined_chan);
                }
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

fn is_join(line: String) -> Result<(bool, String), String> {
    if line.starts_with("$") {
        let parts: Vec<&str> = line.split(" ").collect();
        if parts[0] == JOIN {
            if parts.len() < 2 {
                return Err("join need chan_id".to_string());
            }

            return Ok((true, parts[1].to_string()));
        }
        Ok((false, "".to_string()))
    } else {
        Ok((false, "".to_string()))
    }
}

fn parse_join_resp(line: &String) -> Result<String, String> {
    let parts: Vec<&str> = line.split(": ").collect();
    if parts.len() < 2 {
        return Err("invalid join resp".to_string());
    }

    if line.starts_with("$$") && parts[0] == JOIN_RESP {
        return Ok(parts[1].to_string());
    }

    Err("not join resp".to_string())
}
