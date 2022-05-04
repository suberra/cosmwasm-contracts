use cosmwasm_bignumber::Uint256;
/// Data objects for subscriptions
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Discount Struct to store Discount information per subscriber
/// * `amount`: Discount amount to be applied.
/// * `expiry`: Optional unix timestamp (seconds). If specified, discount is no longer applied when Discount expires
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Discount {
    pub amount: Uint256,
}
