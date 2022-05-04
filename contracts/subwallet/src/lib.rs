pub mod contract;
mod error;
pub mod msg;
mod querier;
pub mod state;
// mod allowance;

pub use crate::error::ContractError;

#[cfg(test)]
mod tests;
