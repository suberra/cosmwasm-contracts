use crate::state::{Agreement, AgreementStatus};
use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{Addr, Binary};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub minimum_interval: u64,
    pub minimum_amount_per_interval: Uint256,
    pub job_registry_contract: Option<String>,
    pub fee_bps: Option<u64>,
    pub fee_address: Option<String>,
    pub max_fee: Option<Uint256>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    CreateAgreement {
        receiver: String,
        amount: Uint256,
        start_at: Option<u64>,
        end_at: Option<u64>,
        interval: u64,
    },
    Transfer {
        agreement_id: u64,
    },
    Work {
        payload: Binary,
    },
    CancelAgreement {
        agreement_id: u64,
    },
    TerminateAgreement {
        agreement_id: u64,
    },
    UpdateConfig {
        job_registry_contract: Option<String>,
        minimum_interval: Option<u64>,
        minimum_amount_per_interval: Option<Uint256>,
        new_owner: Option<String>,
        fee_bps: Option<u64>,
        fee_address: Option<String>,
        max_fee: Option<Uint256>,
    },
    ToggleFreeze {},
    TogglePause {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Agreement {
        agreement_id: u64,
    },
    AgreementsByOwner {
        owner: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    AgreementsByReceiver {
        receiver: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    AllAgreements {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    OverduedAgreements {
        start_after: Option<u64>, // u64 is interval_due_at time
        limit: Option<u32>,
    },
    Config {},
    CanWork {
        payload: Binary,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AgreementResponse {
    pub to: Addr,
    pub from: Addr,
    pub amount: Uint256,
    pub created_at: u64,
    pub interval: u64,
    pub interval_due_at: u64,
    pub last_charged: u64,
    pub start_at: u64,
    pub end_at: Option<u64>,
    pub status: AgreementStatus,
    pub pending_charge: Uint256,
}

impl AgreementResponse {
    pub fn new(
        agreement: &Agreement,
        status: &AgreementStatus,
        pending_charge: &Uint256,
    ) -> AgreementResponse {
        AgreementResponse {
            to: agreement.to.clone(),
            from: agreement.from.clone(),
            amount: agreement.amount,
            created_at: agreement.created_at.seconds(),
            interval: agreement.interval,
            interval_due_at: agreement.interval_due_at.seconds(),
            last_charged: agreement.last_charged.seconds(),
            start_at: agreement.start_at.seconds(),
            end_at: agreement.end_at.map(|e| e.seconds()),
            status: status.clone(),
            pending_charge: *pending_charge,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WorkPayload {
    pub agreement_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AgreementsResponse {
    pub agreement_ids: Vec<u64>,
    pub last_key: Option<u64>,
}
