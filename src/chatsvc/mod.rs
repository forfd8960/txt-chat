use nanoid::nanoid;
use std::collections::{HashMap, HashSet};
use tracing::{info, warn};

use chrono::Utc;
use tokio::sync::broadcast;

pub const JOIN_RESP: &'static str = "$$joined";
pub const LEAVE_RESP: &'static str = "$$leaved";
pub const CREATE_CHAN_RESP: &'static str = "$$create_chan";

#[derive(Debug, Clone)]
pub struct UserInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub chan_id: String,
    pub sender: String,
    pub content: String,
    pub send_time: chrono::DateTime<Utc>,
}

#[derive(Debug)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub online_users: HashMap<String, ()>,
}

pub struct ChatService {
    pub tx: broadcast::Sender<Message>,
    pub users: HashMap<String, UserInfo>,
    pub channels: HashMap<String, Channel>,
    pub user_chans: HashMap<String, HashSet<String>>,
}

impl ChatService {
    pub fn new(cap: usize, tx: broadcast::Sender<Message>) -> Self {
        Self {
            tx,
            users: HashMap::with_capacity(cap),
            channels: HashMap::with_capacity(cap),
            user_chans: HashMap::with_capacity(cap),
        }
    }

    pub fn add_user(&mut self, name: String, uid: String) {
        let user = UserInfo {
            id: uid.clone(),
            name: name.clone(),
        };

        info!("create user: {:?}", user);

        self.users.insert(uid.clone(), user.clone());
        let chan_id = self.create_chan(uid.clone(), name.clone(), Some(uid.clone()));
        info!("user: {} created chan: {}", uid.clone(), chan_id.clone());

        self.send_msg(true, name, chan_id.clone(), chan_id);
    }

    pub fn is_user_sub(&self, uid: &String, chan_id: &String) -> bool {
        let user_chans = self.user_chans.get(uid);
        match user_chans {
            Some(chans) => chans.contains(chan_id),
            None => false,
        }
    }

    pub fn create_chan(
        &mut self,
        uid: String,
        name: String,
        pre_chan_id: Option<String>,
    ) -> String {
        let mut chan = Channel::new(name);
        let mut not_send = false;
        if pre_chan_id.is_some() {
            not_send = true;
            chan.id = pre_chan_id.unwrap();
        }

        let chan_id = chan.id.clone();
        self.channels.insert(chan_id.clone(), chan);

        let mut set = HashSet::new();
        set.insert(chan_id.clone());

        self.user_chans
            .entry(uid.clone())
            .and_modify(|chans| {
                chans.insert(chan_id.clone());
            })
            .or_insert(set);

        if not_send {
            return chan_id;
        }

        if let Some(user) = self.users.get(&uid) {
            self.send_msg(true, user.name.clone(), chan_id.clone(), format!("{}: {}", CREATE_CHAN_RESP, chan_id));
        }

        chan_id
    }

    pub fn join_chan(&mut self, uid: String, chan_id: String) {
        match self.channels.get_mut(&chan_id) {
            Some(chan) => {
                chan.join(uid.clone());

                let mut set = HashSet::new();
                set.insert(chan_id.clone());

                self.user_chans
                    .entry(uid.clone())
                    .and_modify(|chans| {
                        chans.insert(chan_id.clone());
                    })
                    .or_insert(set);

                if let Some(user) = self.users.get(&uid) {
                    self.send_msg(
                        true,
                        user.name.clone(),
                        chan_id.clone(),
                        format!("{}: {}", JOIN_RESP, chan_id),
                    );
                }
            }
            None => info!("chan {} not found", chan_id),
        }
    }

    pub fn leave_chan(&mut self, uid: String, chan_id: String) {
        match self.channels.get_mut(&chan_id) {
            Some(chan) => {
                chan.leave(uid.clone());

                match self.user_chans.get_mut(&uid) {
                    Some(chans) => {
                        chans.remove(&chan_id);
                    }
                    None => {}
                }

                if let Some(user) = self.users.get(&uid) {
                    self.send_msg(
                        true, 
                        user.name.clone(),
                        chan_id.clone(),
                        format!("{}: {}", LEAVE_RESP, chan_id),
                    );
                }

                info!("user: {} leave chan: {}", uid, chan_id);
            }
            None => info!("chan {} not found", chan_id),
        }
    }

    pub fn send_msg(&self, is_cmd: bool, username: String, chan_id: String, msg: String) {
        match self.channels.get(&chan_id) {
            Some(_) => {
                let msg = if is_cmd {
                    msg
                } else {
                    format!("{}: {}", username, msg)
                };

                match self
                    .tx
                    .send(Message::new(username.clone(), chan_id.clone(), msg))
                {
                    Ok(v) => {
                        info!("success send {} message", v);
                    }
                    Err(e) => warn!("failed to send msg to channel: {}, {}", chan_id, e),
                }
                info!("user: {} send msg to: {}", username, chan_id);
            }
            None => {
                info!("chan {} not found", chan_id);
            }
        }
    }
}

impl Channel {
    pub fn new(name: String) -> Self {
        Self {
            id: gen_id(),
            name,
            online_users: HashMap::new(),
        }
    }

    pub fn join(&mut self, user_id: String) {
        self.online_users.insert(user_id, ());
    }

    pub fn leave(&mut self, user_id: String) {
        self.online_users.remove(&user_id);
    }
}

impl Message {
    pub fn new(username: String, chan_id: String, c: String) -> Self {
        Self {
            chan_id,
            sender: username,
            content: c,
            send_time: Utc::now(),
        }
    }

    pub fn to_string(&self) -> String {
        format!("{}", self.content)
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
