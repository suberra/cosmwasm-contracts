use cosmwasm_std::StdError;
use cw0::Expiration;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Cannot set to own account")]
    CannotSetOwnAccount {},

    #[error("No permissions for this account")]
    NotAllowed {},

    #[error("No allowance for this account")]
    NoAllowance {},

    #[error("Program is frozen and transactions for non-owners are not allowed")]
    Frozen {},

    #[error("Insufficient funds")]
    InsufficientFunds {},

    #[error("Message type rejected")]
    MessageTypeRejected {},

    #[error("Invalid parameters")]
    InvalidParams {},

    #[error("Delegate is not allowed")]
    DelegatePerm {},

    #[error("Re-delegate is not allowed")]
    ReDelegatePerm {},

    #[error("Un-delegate is not allowed")]
    UnDelegatePerm {},

    #[error("Withdraw is not allowed")]
    WithdrawPerm {},

    #[error("Set withdraw address is not allowed")]
    WithdrawAddrPerm {},

    #[error("Unsupported message")]
    UnsupportedMessage {},

    #[error("Allowance already expired while setting: {0}")]
    SettingExpiredAllowance(Expiration),
}

impl From<admin_core::ContractError> for ContractError {
    fn from(err: admin_core::ContractError) -> Self {
        match err {
            admin_core::ContractError::Std(error) => ContractError::Std(error),
            admin_core::ContractError::Unauthorized {} => ContractError::Unauthorized {},
            admin_core::ContractError::InvalidParams {} => ContractError::InvalidParams {},
        }
    }
}
