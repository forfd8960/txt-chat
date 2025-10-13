use crate::errors::ChatErrors;

pub mod handler;


#[derive(Debug, Clone)]
pub enum Event {
    Register{username: String}, // reg${{username}}
    CreateChan{user_id: String, chan_name: String}, // create_chan$123$MyChat
    JoinChan{user_id: String, chan_id: String}, // join$123$456
    LeaveChan{user_id: String, chan_id: String}, // leave$123$456
    SendMsg{user_id: String, chan_id: String, msg: String}, // send_msg$123$456$Hello
    Unknown,
}

impl Event {
    pub fn from_string(line: String) -> Result<Self, ChatErrors> {
        let parts: Vec<&str> = line.split("$").into_iter().collect();
        match parts[0] {
            "reg" => {
                if parts.len() < 2 {
                    return Err(ChatErrors::InvalidCommand("reg need username".to_string()));
                }
                Ok(Self::Register { username: parts[1].to_string()})
            }

            "create_chan" => {
                if parts.len() < 3 {
                    return Err(ChatErrors::InvalidCommand("create channel need user id and chan name".to_string()));
                }
                Ok(Self::CreateChan { user_id: parts[1].to_string(), chan_name: parts[2].to_string() })
            }

            "join" => {
                if parts.len() < 3 {
                    return Err(ChatErrors::InvalidCommand("join need user id and chan id".to_string()));
                }
                Ok(Self::JoinChan { user_id: parts[1].to_string(), chan_id: parts[2].to_string() })
            }

            "leave" => {
                if parts.len() < 3 {
                    return Err(ChatErrors::InvalidCommand("leave need user id and chan id".to_string()));
                }
                Ok(Self::LeaveChan { user_id: parts[1].to_string(), chan_id: parts[2].to_string() })
            }

            "send_msg" => {
                if parts.len() < 4 {
                    return Err(ChatErrors::InvalidCommand("send_msg need user id and chan id and msg content".to_string()));
                }
                Ok(Self::SendMsg { user_id: parts[1].to_string(), chan_id: parts[2].to_string() , msg: parts[3].to_string() })
            }
            _ => Err(ChatErrors::CommandNotSupport(parts[0].to_string()))
        }
    }
}

