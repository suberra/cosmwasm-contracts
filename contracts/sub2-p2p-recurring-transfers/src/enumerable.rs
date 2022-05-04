use std::convert::TryInto;

use crate::state::{agreements, AgreementStatus};
use crate::{contract::compute_status, msg::AgreementsResponse};
use cosmwasm_std::{Deps, Env, Order, StdResult};
use cw_storage_plus::{Bound, PrimaryKey, U64Key};

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

/// query_all_agreements_by_owner: Given an owner address, return all
/// the transfers that he/she has made with others
pub fn query_all_agreements_by_owner(
    deps: Deps,
    owner: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<AgreementsResponse> {
    let owner_addr = deps.api.addr_validate(&owner)?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut last_key: u64 = 0u64;
    // transform u64 into bytes key
    let start = start_after.map(U64Key::from).map(Bound::exclusive);
    let agreements: StdResult<Vec<u64>> = agreements()
        .idx
        .owner
        .prefix(owner_addr)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, _v) = item?;
            let id = u64::from_be_bytes(k[..].try_into().expect("unexpected key length"));
            last_key = id;
            Ok(id)
        })
        .collect();

    Ok(AgreementsResponse {
        agreement_ids: agreements?,
        last_key: if last_key > 0 { Some(last_key) } else { None },
    })
}

pub fn query_all_agreements_by_receiver(
    deps: Deps,
    receiver: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<AgreementsResponse> {
    let receiver_addr = deps.api.addr_validate(&receiver)?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut last_key: u64 = 0u64;
    // transform u64 into bytes key
    let start = start_after.map(U64Key::from).map(Bound::exclusive);
    let agreements: StdResult<Vec<u64>> = agreements()
        .idx
        .receiver
        .prefix(receiver_addr)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, _v) = item?;
            let id = u64::from_be_bytes(k[..].try_into().expect("unexpected key length"));
            last_key = id;
            Ok(id)
        })
        .collect();

    Ok(AgreementsResponse {
        agreement_ids: agreements?,
        last_key: if last_key > 0 { Some(last_key) } else { None },
    })
}

pub fn query_all_agreements(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<AgreementsResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let mut last_key: u64 = 0u64;

    // transform u64 into bytes key
    let start = start_after.map(U64Key::from).map(Bound::exclusive);
    let agreement_ids: StdResult<Vec<u64>> = agreements()
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, _v) = item?;
            let id = u64::from_be_bytes(k[..].try_into().expect("unexpected key length"));
            last_key = id;
            Ok(id)
        })
        .collect();

    Ok(AgreementsResponse {
        agreement_ids: agreement_ids?,
        last_key: if last_key > 0 { Some(last_key) } else { None },
    })
}

pub fn query_overdue_agreements(
    deps: Deps,
    env: Env,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<AgreementsResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let now = env.block.time.seconds();
    // as we want to keep the last item (pk) unbounded, we increment time by 1 and use exclusive (below the next tick)
    let max_key = (U64Key::from(now + 1), U64Key::from(0)).joined_key();
    let bound = Bound::Exclusive(max_key);

    let start = start_after
        .map(|since| (U64Key::from(since + 1), U64Key::from(0)).joined_key())
        .map(Bound::exclusive);

    let mut last_key: u64 = 0u64;
    let agreement_ids = agreements()
        .idx
        .due_time
        .range(deps.storage, start, Some(bound), Order::Ascending)
        .filter(|item| match item {
            Ok((_k, v)) => {
                let status = compute_status(v, &env.block);
                status == AgreementStatus::Active
            }
            Err(_) => false,
        })
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            let id = u64::from_be_bytes(k[..].try_into().expect("unexpected key length"));
            last_key = v.interval_due_at.seconds();
            Ok(id)
        })
        .collect::<StdResult<Vec<_>>>()?;

    Ok(AgreementsResponse {
        agreement_ids,
        last_key: if last_key > 0 { Some(last_key) } else { None },
    })
}
