use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub subwallet_code_id: u64,
    pub anchor_market_contract: String,
    pub aterra_token_addr: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    CreateAccount {},
    UpdateConfig {
        new_subwallet_code_id: Option<u64>,
        new_owner: Option<String>,
        new_aterra_token_addr: Option<String>,
        new_anchor_market_contract: Option<String>,
    },
}
