#[cfg(not(feature = "library"))]
use crate::error::ContractError;
use crate::msg::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::query::{
    query_all_streams, query_all_streams_by_receiver, query_all_streams_by_sender,
    query_all_streams_by_token,
};
use crate::state::{increment_stream_id, streams, Stream};
use crate::token::{Asset, AssetInfo};

use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult,
};
use cosmwasm_std::{entry_point, Uint128};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use cw_storage_plus::U64Key;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:p2p_recurring_transfers";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateStream {
            receiver,
            token,
            start_at,
            end_at,
        } => {
            let sender = info.sender.clone();
            let receiver = deps.api.addr_validate(&receiver)?;
            create_stream(deps, env, info, sender, receiver, token, start_at, end_at)
        }
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::Withdraw { stream_id, amount } => withdraw_stream(deps, env, stream_id, amount),
        ExecuteMsg::CancelStream { stream_id } => cancel_stream(deps, env, info, stream_id),
    }
}

/// Called when cw20 token is sent via CW20 Send Msg
pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let token_addr = info.sender.clone();
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::CreateStream {
            receiver,
            token,
            start_at,
            end_at,
        }) => {
            // token contract sent must match
            if !token.info.equal(&AssetInfo::Token {
                contract_addr: token_addr,
            }) {
                return Err(ContractError::TokenMismatch {});
            }

            // amount sent must match
            if token.amount != cw20_msg.amount {
                return Err(ContractError::TokenMismatch {});
            }

            let sender = deps.api.addr_validate(&cw20_msg.sender)?;
            let receiver = deps.api.addr_validate(&receiver)?;

            create_stream(deps, env, info, sender, receiver, token, start_at, end_at)
        }
        Err(err) => Err(ContractError::Std(err)),
    }
}
/// Creates a stream
/// * deps - dependencies
/// * env - environment
/// * info - message info
/// * sender - sender of the stream
/// * receiver - receiver of the stream
/// * token - token to be streamed, can be native or CW20 token
/// * start_at - start time of the stream
/// * end_at - end time of the stream
///
/// throws an error when
/// * receiver is contract or sender is receiver
/// * token amount is zero
/// * start_at is greater than or equal to end_at
/// * native token sent is different from token amount
/// * amount is not a multiple of duration
#[allow(clippy::too_many_arguments)]
pub fn create_stream(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: Addr,
    receiver: Addr,
    token: Asset,
    may_start_at: Option<u64>,
    end_at: u64,
) -> Result<Response, ContractError> {
    // Cannot send to self or contract
    if receiver == sender {
        return Err(ContractError::InvalidParam {
            name: "receiver".to_string(),
            message: "cannot send to self".to_string(),
        });
    }

    if receiver == env.contract.address {
        return Err(ContractError::InvalidParam {
            name: "receiver".to_string(),
            message: "cannot send to contract".to_string(),
        });
    }

    let start_at = may_start_at.unwrap_or_else(|| env.block.time.seconds());

    if start_at < env.block.time.seconds() {
        return Err(ContractError::InvalidParam {
            name: "start_at".to_string(),
            message: "cannot be in the past".to_string(),
        });
    }

    // Duration cannot be 0 or negative
    if end_at <= start_at {
        return Err(ContractError::InvalidParam {
            name: "end_at".to_string(),
            message: "cannot be earlier or equal to start_at".to_string(),
        });
    }

    let duration = end_at - start_at;

    // Amount cannot be 0
    let amount = token.amount;
    if amount.is_zero() {
        return Err(ContractError::InvalidParam {
            name: "amount".to_string(),
            message: "cannot be 0".to_string(),
        });
    }

    // Ensure native token sent
    token.assert_sent_native_token_balance(&info)?;

    let rate_per_second = Decimal::from_ratio(amount, Uint128::from(duration));

    let stream_id = increment_stream_id(deps.storage)?;

    let stream = Stream {
        receiver,
        sender,
        token: token.clone(),
        remaining_amount: token.amount,
        start_at,
        end_at,
        rate_per_second,
    };

    streams().save(deps.storage, U64Key::from(stream_id), &stream)?;

    Ok(Response::new().add_attributes(vec![
        attr("method", "create_stream"),
        attr("stream_id", stream_id.to_string()),
    ]))
}

pub fn withdraw_stream(
    deps: DepsMut,
    env: Env,
    stream_id: u64,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let key = U64Key::from(stream_id);
    let mut stream = match streams().may_load(deps.storage, key.clone())? {
        Some(stream) => stream,
        None => return Err(ContractError::StreamNotFound {}),
    };

    let balance = compute_balance_of(&stream, &stream.receiver, env.block.time.seconds())?;

    let withdrawal_amount = if let Some(amount) = amount {
        amount
    } else {
        // withdraws all balance when no amount is given
        balance
    };

    if withdrawal_amount.is_zero() {
        return Err(ContractError::ZeroTransferableAmount {});
    }

    if balance < withdrawal_amount {
        return Err(ContractError::InsufficientBalance {});
    }

    stream.remaining_amount -= withdrawal_amount;

    if stream.remaining_amount.is_zero() {
        streams().remove(deps.storage, key)?;
    } else {
        streams().save(deps.storage, key, &stream)?;
    }

    let transfer_asset = Asset {
        amount: withdrawal_amount,
        info: stream.token.info,
    };

    Ok(Response::new()
        .add_message(transfer_asset.into_msg(deps.as_ref(), stream.receiver)?)
        .add_attributes(vec![
            attr("method", "stream_withdraw"),
            attr("stream_id", stream_id.to_string()),
        ]))
}

pub fn cancel_stream(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    stream_id: u64,
) -> Result<Response, ContractError> {
    let stream = match streams().may_load(deps.storage, U64Key::from(stream_id))? {
        Some(stream) => stream,
        None => return Err(ContractError::StreamNotFound {}),
    };

    stream.assert_sender_or_receiver(&info.sender)?;

    let current_time = env.block.time.seconds();
    let sender_balance = compute_balance_of(&stream, &stream.sender, current_time)?;
    let receiver_balance = compute_balance_of(&stream, &stream.receiver, current_time)?;

    let mut messages = vec![];
    let sender_asset = Asset {
        amount: sender_balance,
        info: stream.token.info.clone(),
    };

    let receiver_asset = Asset {
        amount: receiver_balance,
        info: stream.token.info,
    };

    if receiver_asset.amount > Uint128::zero() {
        messages.push(receiver_asset.into_msg(deps.as_ref(), stream.receiver)?);
    }

    if sender_asset.amount > Uint128::zero() {
        messages.push(sender_asset.into_msg(deps.as_ref(), stream.sender)?);
    }

    streams().remove(deps.storage, U64Key::from(stream_id))?;

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        attr("method", "cancel_stream"),
        attr("stream_id", stream_id.to_string()),
    ]))
}

fn compute_time_delta(stream: &Stream, current_time: u64) -> u64 {
    // Not started
    if current_time <= stream.start_at {
        return 0;
    }
    // Stream in progress
    if current_time < stream.end_at {
        return current_time - stream.start_at;
    }

    // Stream completed
    stream.end_at - stream.start_at
}
fn compute_balance_of(stream: &Stream, address: &Addr, current_time: u64) -> StdResult<Uint128> {
    let delta = compute_time_delta(stream, current_time);
    let duration = stream.end_at - stream.start_at;

    // % streamed * amount, truncates decimals
    let mut streamed_amount = Decimal::from_ratio(delta, duration) * stream.token.amount;

    // Factor in previously withdrawn amounts
    let withdrawn_amount = stream.token.amount - stream.remaining_amount;
    if !withdrawn_amount.is_zero() {
        streamed_amount -= withdrawn_amount;
    }

    if *address == stream.receiver {
        return Ok(streamed_amount);
    }

    if *address == stream.sender {
        return Ok(stream.remaining_amount - streamed_amount);
    }

    Ok(Uint128::zero())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Stream { stream_id } => to_binary(&query_stream(deps, stream_id)?),
        QueryMsg::BalanceOf { stream_id, address } => {
            to_binary(&query_stream_balance_of(deps, env, stream_id, address)?)
        }
        QueryMsg::StreamsBySender {
            sender,
            start_after,
            limit,
        } => to_binary(&query_all_streams_by_sender(
            deps,
            sender,
            start_after,
            limit,
        )?),
        QueryMsg::StreamsByReceiver {
            receiver,
            start_after,
            limit,
        } => to_binary(&query_all_streams_by_receiver(
            deps,
            receiver,
            start_after,
            limit,
        )?),
        QueryMsg::StreamsByToken {
            token_info,
            start_after,
            limit,
        } => to_binary(&query_all_streams_by_token(
            deps,
            token_info,
            start_after,
            limit,
        )?),
        QueryMsg::AllStreams { start_after, limit } => {
            to_binary(&query_all_streams(deps, start_after, limit)?)
        }
    }
}

pub fn query_stream(deps: Deps, stream_id: u64) -> StdResult<Stream> {
    streams().load(deps.storage, U64Key::from(stream_id))
}

pub fn query_stream_balance_of(
    deps: Deps,
    env: Env,
    stream_id: u64,
    address: String,
) -> StdResult<Uint128> {
    let address = deps.api.addr_validate(&address)?;
    let stream = streams().load(deps.storage, U64Key::from(stream_id))?;
    let balance = compute_balance_of(&stream, &address, env.block.time.seconds())?;
    Ok(balance)
}

#[allow(dead_code)]
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}
