use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChatErrors {
    #[error("invalid cmd: {0}")]
    InvalidCommand(String),

    #[error("cmd: {0} is not support")]
    CommandNotSupport(String),

    #[error("not set current channel yet")]
    UnknownCurrentChan,
}
