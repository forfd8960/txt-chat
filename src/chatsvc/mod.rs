use nanoid::nanoid;
use std::collections::HashMap;
use tracing::{info, warn};

use chrono::Utc;
use tokio::sync::{Mutex, RwLock, broadcast};

#[derive(Debug, Clone)]
pub struct UserInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub sender: String,
    pub content: String,
    pub send_time: chrono::DateTime<Utc>,
}

#[derive(Debug)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub sender: broadcast::Sender<Message>,
    pub online_users: HashMap<String, broadcast::Receiver<Message>>,
}

pub struct ChatService {
    pub users: HashMap<String, UserInfo>,
    pub channels: HashMap<String, Channel>,
}

impl ChatService {
    pub fn new(cap: usize) -> Self {
        Self {
            users:HashMap::with_capacity(cap),
            channels: HashMap::with_capacity(cap),
        }
    }

    pub fn add_user(&mut self, name: String) -> UserInfo {
        let uid = gen_id();
        let user = UserInfo {
            id: uid.clone(),
            name,
        };

        self.users.insert(uid, user.clone());
        user
    }

    pub fn create_chan(&mut self, _: String, name: String) -> String {
        let chan = Channel::new(name);
        let chan_id = chan.id.clone();
        self.channels.insert(chan_id.clone(), chan);
        chan_id
    }

    pub fn join_chan(&mut self, uid: String, chan_id: String) {
        match self.channels.get_mut(&chan_id) {
            Some(chan) => chan.join(uid),
            None => info!("chan {} not found", chan_id),
        }
    }

    pub fn leave_chan(&mut self, uid: String, chan_id: String) {
        match self.channels.get_mut(&chan_id) {
            Some(chan) => chan.leave(uid),
            None => info!("chan {} not found", chan_id),
        }
    }

    pub fn send_msg(&mut self, uid: String, chan_id: String, msg: String) -> String {
        match self.channels.get_mut(&chan_id) {
            Some(chan) => {
                chan.send_msg(Message::new(uid.clone(), msg));
                info!("user: {} send msg to: {}", uid, chan_id);
                "OK".to_string()
            }
            None => {
                info!("chan {} not found", chan_id);
                "ChanNotFound".to_string()
            },
        }
    }
}

impl Channel {
    pub fn new(name: String) -> Self {
        let (tx, _) = broadcast::channel::<Message>(1000);
        Self {
            id: gen_id(),
            name,
            sender: tx,
            online_users: HashMap::new(),
        }
    }

    pub fn join(&mut self, user_id: String) {
        let rx = self.sender.clone().subscribe();
        self.online_users.insert(user_id, rx);
    }

    pub async fn recv_msg(&mut self, user_id: String) {
        match self.online_users.get_mut(&user_id) {
            Some(rx) => while let Ok(msg) = rx.recv().await {
                //send msg into Tcp connection.
                info!("sending msg to: {}", user_id);
            },
            None => warn!("rx not found"),
        }
    }

    pub fn leave(&mut self, user_id: String) {
        self.online_users.remove(&user_id);
    }

    pub fn send_msg(&mut self, msg: Message) {
        match self.sender.send(msg) {
            Ok(_) => info!("success send msg to chan: {}", self.id),
            Err(e) => warn!("send msg failed: {}", e),
        }
    }
}

impl Message {
    pub fn new(uid: String, c: String) -> Self {
        Self {
            sender: uid,
            content: c,
            send_time: Utc::now(),
        }
    }

    pub fn to_string(&self) -> String {
        format!("{}({}): {}", self.sender, self.send_time.to_string(), self.content)
    }
}

impl UserInfo {
    pub fn to_string(&self) -> String {
        format!("{}: {}", self.id, self.name)
    }
}

fn gen_id() -> String {
    let alphabet: [char; 16] = [
        '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', 'a', 'b', 'c', 'd', 'e', 'f',
    ];

    nanoid!(10, &alphabet) //=> "4f90d13a42"
}
