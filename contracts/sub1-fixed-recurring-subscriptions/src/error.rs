use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Unable to charge")]
    CannotCharge {},

    #[error("Nothing to charge")]
    NoCharge {},

    #[error("Invalid Param")]
    InvalidParam {},

    #[error("Invalid Fee")]
    InvalidFee {},

    #[error("Invalid discount")]
    InvalidDiscount {},

    #[error("Subscription not found")]
    SubscriptionNotFound {},

    #[error("Subscription found")]
    ExistingSubscriptionFound {},

    #[error("No job registry found")]
    NoJobRegistry {},

    #[error("Operation not permitted. Contract is paused.")]
    Paused {},

    #[error("Operation not permitted. Contract is not paused.")]
    Unpaused {},

    #[error("Subscription cancelled")]
    SubscriptionCancelled {},
}
