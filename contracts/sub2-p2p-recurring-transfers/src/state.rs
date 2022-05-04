use cosmwasm_bignumber::Uint256;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, StdResult, Storage, Timestamp};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex, U64Key};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    // flag to control if contract can receive more agreements creation
    pub is_paused: bool,
    /// once the contract is frozen, then no further transfers can be made and no agreements can be created. Intended to be a circuit-breaker measure
    pub is_frozen: bool,
    pub job_registry_contract: Option<Addr>,
    pub minimum_interval: u64,
    pub minimum_amount_per_interval: Uint256,
    pub fee_bps: u64,
    pub fee_address: Addr,
    pub max_fee: Uint256,
}

pub const CONFIG: Item<Config> = Item::new("config");

// Incremental agreement_id, up only
pub const AGREEMENT_ID: Item<u64> = Item::new("agreement_id");
pub fn agreement_id(storage: &dyn Storage) -> StdResult<u64> {
    Ok(AGREEMENT_ID.may_load(storage)?.unwrap_or_default())
}

pub fn increment_agreement_id(storage: &mut dyn Storage) -> StdResult<u64> {
    let val = agreement_id(storage)? + 1;
    AGREEMENT_ID.save(storage, &val)?;
    Ok(val)
}

// Indexed by owner addr & receiver
pub struct AgreementsIndexes<'a> {
    pub owner: MultiIndex<'a, (Addr, Vec<u8>), Agreement>, // Allows iteration over owners
    pub receiver: MultiIndex<'a, (Addr, Vec<u8>), Agreement>, // Allows iteration over receiver
    pub due_time: MultiIndex<'a, (U64Key, Vec<u8>), Agreement>, // Allows iteration sorted by overdue_time
}

impl<'a> IndexList<Agreement> for AgreementsIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Agreement>> + '_> {
        let v: Vec<&dyn Index<Agreement>> = vec![&self.owner, &self.receiver, &self.due_time];
        Box::new(v.into_iter())
    }
}

// U64key refers to agreement_id pk
pub fn agreements<'a>() -> IndexedMap<'a, U64Key, Agreement, AgreementsIndexes<'a>> {
    let indexes = AgreementsIndexes {
        owner: MultiIndex::new(
            |d: &Agreement, k: Vec<u8>| (d.from.clone(), k),
            "recurring_agreements",
            "recurring_agreements__owner",
        ),
        receiver: MultiIndex::new(
            |d: &Agreement, k: Vec<u8>| (d.to.clone(), k),
            "recurring_agreements",
            "recurring_agreements__receiver",
        ),
        due_time: MultiIndex::new(
            |d: &Agreement, k: Vec<u8>| (d.interval_due_at.seconds().into(), k),
            "recurring_agreements",
            "recurring_agreements__duetime",
        ),
    };
    IndexedMap::new("recurring_agreements", indexes)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum AgreementStatus {
    NotStarted,
    Active,
    Expired,
    Lapsed,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Agreement {
    pub to: Addr,
    pub from: Addr,
    pub amount: Uint256,
    pub created_at: Timestamp,
    pub interval: u64,
    pub interval_due_at: Timestamp,
    pub last_charged: Timestamp,
    pub start_at: Timestamp,
    pub end_at: Option<Timestamp>,
}
