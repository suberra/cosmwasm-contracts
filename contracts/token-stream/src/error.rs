use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Insufficient balance")]
    InsufficientBalance {},

    #[error("Invalid parameters: {name} {message}")]
    InvalidParam { name: String, message: String },

    #[error("Invalid receiver")]
    InvalidReceiver {},

    #[error("Zero transferable amount")]
    ZeroTransferableAmount {},

    #[error("No stream found")]
    StreamNotFound {},

    #[error("Token sent mismatch")]
    TokenMismatch {},
}
