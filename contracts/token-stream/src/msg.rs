use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::token::{Asset, AssetInfo};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    CreateStream {
        receiver: String,
        token: Asset,
        start_at: Option<u64>,
        end_at: u64,
    },
    Withdraw {
        stream_id: u64,
        amount: Option<Uint128>,
    },
    CancelStream {
        stream_id: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    CreateStream {
        receiver: String,
        token: Asset,
        start_at: Option<u64>,
        end_at: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Stream {
        stream_id: u64,
    },
    BalanceOf {
        stream_id: u64,
        address: String,
    },
    StreamsBySender {
        sender: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    StreamsByReceiver {
        receiver: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    StreamsByToken {
        token_info: AssetInfo,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    AllStreams {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StreamsResponse {
    pub stream_ids: Vec<u64>,
    pub last_key: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
