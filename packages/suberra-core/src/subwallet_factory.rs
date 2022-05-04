use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Configurations for SubwalletFactory. Required for query from Subwallets
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SubwalletFactoryConfig {
    pub subwallet_code_id: u64,
    pub contract_owner: Addr,
    pub anchor_market_contract: Addr,
    pub aterra_token_addr: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    GetSubwalletAddress { owner_address: String },
}
