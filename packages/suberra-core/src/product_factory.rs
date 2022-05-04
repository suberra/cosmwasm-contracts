use cosmwasm_bignumber::Uint256;
use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// ## Description
/// This structure describes the basic settings for creating a contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub product_code_id: u64,
    pub protocol_fee_bps: u64,
    pub min_protocol_fee: Uint256,
    pub min_amount_per_interval: Uint256,
    pub min_unit_interval_hour: u64,
    pub fee_address: String,
    pub job_registry_address: String,
}

/// ## Description
/// This structure describes the execute messages of the contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    CreateProduct {
        product_info: CreateProductExecuteMsg,
    },
    UpdateConfig {
        new_owner: Option<String>,
        new_is_restricted: Option<bool>,
        new_product_code_id: Option<u64>,
        new_protocol_fee_bps: Option<u64>,
        new_min_protocol_fee: Option<Uint256>,
        new_min_amount_per_interval: Option<Uint256>,
        new_min_unit_interval_hour: Option<u64>,
        new_fee_address: Option<String>,
        new_job_registry_address: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Config returns the settigs that specified in custom [`ConfigResponse`]
    Config {},
    /// Returns a list of contracts created by owner
    ProductsByOwner {
        owner: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
}

/// # Description
/// Execute message used for creating a product through the product_factory
// - receiver_address: address that will receive the payments
// - unit_amount: Amount to be charged in every billing cycle
// - initial_amount: initial_amount that must be transferred to the receiver for the subscription to be created. Common in most services
// - unit_interval_hour: Duration of the billing cycle in hours
// - max_amount_chargeable: Maximum amount that will be chargeable to the subscriber.
// - additional_grace_period_hour: Amount of time (in hours) that a subscription should still be active despite payment is due
// - uri : Metadata for the subscription
// - admins: List of admins that have the rights to manage some features of the product contracts
// - mutable: States if the contract is mutable

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CreateProductExecuteMsg {
    pub receiver_address: String,
    pub unit_amount: Uint256,
    pub initial_amount: Uint256,
    pub unit_interval_hour: u64,
    pub max_amount_chargeable: Option<Uint256>,
    pub additional_grace_period_hour: Option<u64>,
    pub uri: String,
    pub admins: Vec<String>,
    pub mutable: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    /// The Contract address that used for controls settings for factory, pools and tokenomics contracts
    pub owner: String,
    /// boolean to indicate if the contract is restricted.
    pub is_restricted: bool,
    /// Code identifier of the product
    pub product_code_id: u64,
    /// protocol fee in basis points
    pub protocol_fee_bps: u64,
    /// minimum protocol fee,
    pub min_protocol_fee: Uint256,
    /// minimum amount per interval
    pub min_amount_per_interval: Uint256,
    /// minimum unit interval in hours
    pub min_unit_interval_hour: u64,
    /// fee address
    pub fee_address: String,
    /// job registry address
    pub job_registry_address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ProductsResponse {
    pub products: Vec<Addr>,
    pub last_key: Option<u64>,
}
