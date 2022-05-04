use crate::state::SubscriptionInfo;
use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{Binary, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use suberra_core::subscriptions::Discount;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateConfig {
        receiver_address: Option<String>,
        additional_grace_period_hour: Option<u64>,
        initial_amount: Option<Uint256>,
        uri: Option<String>,
    },
    UpdateAdmins {
        admins: Vec<String>,
    },
    Subscribe {},
    Cancel {},
    TogglePause {},
    ToggleFreeze {},
    RemoveSubscriber {
        subscriber: String,
    },
    ModifySubscriber {
        new_created_at: Option<u64>,
        new_last_charged: Option<u64>,
        new_interval_end_at: Option<u64>,
        subscriber: String,
    },
    SetDiscount {
        discount: Option<Discount>,
        subscriber: String,
    },
    Charge {
        payer_address: String,
    },
    Work {
        payload: Binary,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubWalletExecuteMsg {
    TransferUST { amount: Uint128, recipient: String },
    TransferAToken { amount: Uint128, recipient: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobsRegistryExecuteMsg {
    WorkReceipt { worker_address: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AdminConfig {},
    Owner {},
    Config {},
    Subscription {
        subscriber: String,
    },
    Subscriptions {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    CanWork {
        payload: Binary,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WorkPayload {
    pub payer_address: String,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner_address: String,
    pub receiver_address: String,
    pub unit_interval_seconds: u64,
    pub unit_amount: Uint256,
    pub additional_grace_period: u64,
    pub initial_amount: Uint256,
    pub is_paused: bool,
    pub is_frozen: bool,
    pub uri: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SubscriptionInfoResponse {
    pub subscriber: String,
    pub created_at: u64,
    pub interval_end_at: u64,
    pub last_charged: u64,
    pub is_cancelled: bool,
    pub is_active: bool,
    pub discount_per_interval: Option<Discount>,
    pub amount_chargeable: Option<Uint256>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SubscriptionsResponse {
    pub subscriptions: Vec<SubscriptionInfo>,
}
