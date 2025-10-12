use std::sync::Arc;

use tokio::sync::RwLock;

use crate::{chatsvc::ChatService, event::Event};

pub async fn handle_event(svc: Arc<RwLock<ChatService>>, event: Event) -> String {
    let mut svc = svc.write().await;

    match event {
        Event::Register { username } => svc.add_user(username).to_string(),
        Event::CreateChan { user_id, chan_name } => svc.create_chan(user_id, chan_name),
        Event::JoinChan { user_id, chan_id } => {
            svc.join_chan(user_id, chan_id);
            "OK".to_string()
        }
        Event::LeaveChan { user_id, chan_id } => {
            svc.leave_chan(user_id, chan_id);
            "OK".to_string()
        }
        Event::SendMsg {
            user_id,
            chan_id,
            msg,
        } => svc.send_msg(user_id, chan_id, msg),
        Event::Unknown => "OK".to_string(),
    }
}
