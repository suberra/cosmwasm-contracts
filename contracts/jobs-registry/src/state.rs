use cosmwasm_std::Addr;
use cosmwasm_std::Coin;
use cw0::NativeBalance;
use cw_storage_plus::Item;
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub base_fee: Vec<Coin>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Job {
    pub owner: Addr,
    pub name: String,
    pub contract: Addr,
    pub active: bool,
    pub job_id: u64,
}

pub const CONFIG: Item<Config> = Item::new("\u{0}\u{6}config");

pub const COUNT: Item<u64> = Item::new("\u{0}\u{5}count");

pub const JOBS: Map<&Addr, Job> = Map::new("jobs");

pub const CREDITS: Map<&Addr, NativeBalance> = Map::new("credits");
