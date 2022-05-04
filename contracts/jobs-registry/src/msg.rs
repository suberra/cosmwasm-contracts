use crate::state::Job;
use cosmwasm_std::Addr;
use cosmwasm_std::Coin;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AddJob {
        contract_address: String,
        name: String,
    },
    RemoveJob {
        contract_address: String,
    },
    AddCredits {
        contract_address: String,
    },
    WorkReceipt {
        worker_address: String,
    },
    SetBaseFee {
        base_fee: Vec<Coin>,
    },
    UpdateAdmins {
        admins: Vec<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetConfig {},
    NumJobs {},
    GetJobCredits {
        contract_address: String,
    },
    GetJob {
        contract_address: String,
    },
    AllJobs {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub owner: String,
    pub admins: Vec<String>,
    pub base_fee: Vec<Coin>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CountResponse {
    pub count: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct JobInfo {
    pub is_active: bool,
    pub contract_address: Addr,
    pub job_id: u64,
    pub owner: Addr,
    pub name: String,
}

impl From<Job> for JobInfo {
    fn from(job: Job) -> Self {
        JobInfo {
            is_active: job.active,
            contract_address: job.contract,
            job_id: job.job_id,
            owner: job.owner,
            name: job.name,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AllJobsResponse {
    pub jobs: Vec<JobInfo>,
}
