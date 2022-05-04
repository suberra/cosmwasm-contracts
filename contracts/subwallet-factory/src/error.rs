use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Existing Subwallet")]
    ExistingSubwallet {},

    #[error("Invalid reply ID")]
    InvalidReplyID {},

    #[error("Unexpected Error")]
    UnexpectedError {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
