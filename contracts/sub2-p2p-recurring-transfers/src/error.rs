use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Cannot terminate active agreement")]
    CannotTerminateActiveAgreement {},

    #[error("Invalid parameters")]
    InvalidParams {},

    #[error("Invalid end time")]
    InvalidEndtime {},

    #[error("Invalid fee bps")]
    InvalidFee {},

    #[error("Cannot set own account")]
    CannotSetOwnAccount {},

    #[error("Agreement not found")]
    AgreementNotFound {},

    #[error("Zero transferable amount")]
    ZeroTransferableAmount {},

    #[error("No job registry found")]
    NoJobRegistry {},

    #[error("P2P contract is frozen.")]
    Frozen {},

    #[error("P2P contract is paused.")]
    Paused {},
}
