use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Decimal, StdResult, Storage, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex, U64Key};

use crate::{token::Asset, ContractError};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");

// Incremental stream_id, up only
pub const STREAM_ID: Item<u64> = Item::new("stream_id");
pub fn stream_id(storage: &dyn Storage) -> StdResult<u64> {
    Ok(STREAM_ID.may_load(storage)?.unwrap_or_default())
}

pub fn increment_stream_id(storage: &mut dyn Storage) -> StdResult<u64> {
    let val = stream_id(storage)? + 1;
    STREAM_ID.save(storage, &val)?;
    Ok(val)
}

// Indexed by owner addr & receiver
pub struct StreamIndexes<'a> {
    pub sender: MultiIndex<'a, (Addr, Vec<u8>), Stream>, // Allows iteration over senders
    pub receiver: MultiIndex<'a, (Addr, Vec<u8>), Stream>, // Allows iteration over receiver
    pub token: MultiIndex<'a, (Vec<u8>, Vec<u8>), Stream>, // Allows iteration over token
}

impl<'a> IndexList<Stream> for StreamIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Stream>> + '_> {
        let v: Vec<&dyn Index<Stream>> = vec![&self.sender, &self.receiver, &self.token];
        Box::new(v.into_iter())
    }
}

// U64key refers to stream_id pk
pub fn streams<'a>() -> IndexedMap<'a, U64Key, Stream, StreamIndexes<'a>> {
    let indexes = StreamIndexes {
        sender: MultiIndex::new(
            |d: &Stream, k: Vec<u8>| (d.sender.clone(), k),
            "token_streams",
            "token_streams__sender",
        ),
        receiver: MultiIndex::new(
            |d: &Stream, k: Vec<u8>| (d.receiver.clone(), k),
            "token_streams",
            "token_streams__receiver",
        ),
        token: MultiIndex::new(
            |d: &Stream, k: Vec<u8>| (d.token.info.as_bytes().to_vec(), k),
            "token_streams",
            "token_streams__token",
        ),
    };
    IndexedMap::new("token_streams", indexes)
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Stream {
    pub sender: Addr,
    pub receiver: Addr,
    // Streamed asset, amount reflects starting deposits
    pub token: Asset,
    pub rate_per_second: Decimal,
    pub remaining_amount: Uint128,
    pub start_at: u64,
    pub end_at: u64,
}

impl Stream {
    pub fn assert_sender_or_receiver(&self, address: &Addr) -> Result<(), ContractError> {
        if *address != self.sender && *address != self.receiver {
            return Err(ContractError::Unauthorized {});
        }
        Ok(())
    }
}
