use cosmwasm_bignumber::Uint256;
use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct SubwalletInstantiateMsg {
    pub admins: Vec<String>,
    pub mutable: bool,
    pub stable_denom: String,
    pub owner_address: String,
    pub subwallet_factory_addr: String
}

//  - receiver_address: address that will receive the payments
//  - job_registry_contact: Contract address of the job_registry. Required for automation
//  - unit_amount: Amount to be charged in every billing cycle
//  - initial_amount: initial_amount that must be transferred to the receiver for the subscription to be created. Common in most services
//  - unit_interval_hour: Duration of the billing cycle in hours
//  - max_amount_chargeable: Maximum amount that will be chargeable to the subscriber.
//  - additional_grace_period_hour: Amount of time (in hours) that a subscription should still be active despite payment is due
//  - uri : Metadata for the subscription
//  - admins: List of admins that have the rights to manage some features of the product contracts
//  - mutable: States if the contract is mutable
//  - factory_address: Stores the address of the factory that instantiates the contract

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ProductInstantiateMsg {
    pub receiver_address: String,
    pub unit_amount: Uint256,
    pub initial_amount: Uint256,
    pub unit_interval_hour: u64,
    pub additional_grace_period_hour: Option<u64>,
    pub uri: String,
    pub owner: String,
    pub admins: Vec<String>,
    pub mutable: bool,
    pub factory_address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubWalletExecuteMsg {
    TransferUST { amount: Uint128, recipient: String },
    TransferAToken { amount: Uint128, recipient: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct JobsRegistryInstantiateMsg {}

/// JobRegistryExecuteMsg: Work receipt is called when a worker calls a function and the function is successfully executed and terminated
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobsRegistryExecuteMsg {
    WorkReceipt {
        worker_address: String,
    },
    AddJob {
        contract_address: String,
        name: String,
    },
}

/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
