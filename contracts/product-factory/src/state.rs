use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::{Item, Map, U64Key};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// ## Description
/// This structure describes the main control config of factory.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// The Contract address that used for controls settings for factory, pools and tokenomics contracts
    pub owner: Addr,
    /// Code identifier of the product
    pub product_code_id: u64,
    /// protocol fee in basis points. If set, this charges a certain percentage on the amount transacted for all subscriptions contract
    pub protocol_fee_bps: u64,
    /// minimum protocol fee that will be collectable from the subscription contract.
    pub min_protocol_fee: Uint256,
    /// minimum amount per interval in UST
    pub min_amount_per_interval: Uint256,
    /// address that will be receiving the protocol fees
    pub fee_address: Addr,
    /// address for the job registry contract
    pub job_registry_address: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");

/// Incremental product ID that is stored while the product subscription is being created
pub const PRODUCT_ID: Item<u64> = Item::new("product_id");

/// returns the current product_id
pub fn product_id(storage: &dyn Storage) -> StdResult<u64> {
    Ok(PRODUCT_ID.may_load(storage)?.unwrap_or_default())
}

/// increase the product_id
pub fn increment_product_id(storage: &mut dyn Storage) -> StdResult<u64> {
    let val = product_id(storage)? + 1;
    PRODUCT_ID.save(storage, &val)?;
    Ok(val)
}

/// Saves mapping between (Owner, ProductID) to address
pub const PRODUCTS: Map<(Addr, U64Key), Addr> = Map::new("products");
