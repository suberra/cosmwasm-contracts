use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Job already exist")]
    JobExist,

    #[error("Job not found")]
    JobNotFound,

    #[error("Job is inactive")]
    JobNotActive,

    #[error("Job has insufficient credits")]
    JobInsufficientCredits,

    #[error("Invalid Param")]
    InvalidParam,
}
