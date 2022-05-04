use std::convert::TryInto;

use crate::{
    msg::StreamsResponse,
    state::{streams, Stream},
    token::AssetInfo,
};
use cosmwasm_std::{Addr, Deps, Order, StdResult};
use cw_storage_plus::{Bound, MultiIndex, U64Key};

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub fn query_all_streams_by_sender(
    deps: Deps,
    sender: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<StreamsResponse> {
    query_all_streams_by_address(deps, streams().idx.sender, sender, start_after, limit)
}

pub fn query_all_streams_by_receiver(
    deps: Deps,
    sender: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<StreamsResponse> {
    query_all_streams_by_address(deps, streams().idx.receiver, sender, start_after, limit)
}

pub fn query_all_streams_by_token(
    deps: Deps,
    token_info: AssetInfo,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<StreamsResponse> {
    let start = start_after.map(U64Key::from).map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut last_key: u64 = 0u64;

    let streams: StdResult<Vec<u64>> = streams()
        .idx
        .token
        .prefix(token_info.as_bytes().to_vec())
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, _v) = item?;
            let id = u64::from_be_bytes(k[..].try_into().expect("unexpected key length"));
            last_key = id;
            Ok(id)
        })
        .collect();

    Ok(StreamsResponse {
        stream_ids: streams?,
        last_key: if last_key > 0 { Some(last_key) } else { None },
    })
}

pub fn query_all_streams_by_address(
    deps: Deps,
    index: MultiIndex<(Addr, Vec<u8>), Stream>,
    address: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<StreamsResponse> {
    let address_addr = deps.api.addr_validate(&address)?;
    let start = start_after.map(U64Key::from).map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut last_key: u64 = 0u64;

    let streams: StdResult<Vec<u64>> = index
        .prefix(address_addr)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, _v) = item?;
            let id = u64::from_be_bytes(k[..].try_into().expect("unexpected key length"));
            last_key = id;
            Ok(id)
        })
        .collect();

    Ok(StreamsResponse {
        stream_ids: streams?,
        last_key: if last_key > 0 { Some(last_key) } else { None },
    })
}

pub fn query_all_streams(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<StreamsResponse> {
    let start = start_after.map(U64Key::from).map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let mut last_key: u64 = 0u64;
    let streams: StdResult<Vec<u64>> = streams()
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, _v) = item?;
            let id = u64::from_be_bytes(k[..].try_into().expect("unexpected key length"));
            last_key = id;
            Ok(id)
        })
        .collect();

    Ok(StreamsResponse {
        stream_ids: streams?,
        last_key: if last_key > 0 { Some(last_key) } else { None },
    })
}
