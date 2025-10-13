use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::info;

use crate::{chatsvc::ChatService, event::Event};

pub async fn handle_event(uid: String, svc: Arc<RwLock<ChatService>>, event: Event) {
    info!("start handle event");
    let mut svc = svc.write().await;
    info!("lock service...");

    match event {
        Event::Register { username } => {
            info!("adding user to service");
            svc.add_user(username, uid);
        }
        Event::CreateChan { user_id, chan_name } => {svc.create_chan(user_id, chan_name);},
        Event::JoinChan { user_id, chan_id } => svc.join_chan(user_id, chan_id),
        Event::LeaveChan { user_id, chan_id } => svc.leave_chan(user_id, chan_id),
        Event::SendMsg {
            user_id,
            chan_id,
            msg,
        } => svc.send_msg(user_id, chan_id, msg),
        Event::Unknown => {},
    }
}
