use crate::enumerable::{
    query_all_agreements, query_all_agreements_by_owner, query_all_agreements_by_receiver,
    query_overdue_agreements,
};
use crate::error::ContractError;
use crate::msg::{AgreementResponse, ExecuteMsg, InstantiateMsg, QueryMsg, WorkPayload};
use crate::state::{
    agreements, increment_agreement_id, Agreement, AgreementStatus, Config, CONFIG,
};
use cosmwasm_bignumber::{Decimal256, Uint256};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, Attribute, Binary, BlockInfo, CosmosMsg, Deps, DepsMut,
    Env, MessageInfo, Response, StdResult, Timestamp, WasmMsg,
};
use cw2::set_contract_version;
use cw_storage_plus::U64Key;
use suberra_core::msg::{JobsRegistryExecuteMsg, MigrateMsg, SubWalletExecuteMsg};
use suberra_core::util::optional_addr_validate;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:p2p_recurring_transfers";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let init_msg = msg.clone();

    let job_registry_contract = match init_msg.job_registry_contract {
        Some(job_contract) => Some(deps.api.addr_validate(&job_contract)?),
        None => None,
    };

    // Fee address defaults to owner
    let fee_address =
        optional_addr_validate(deps.api, msg.fee_address)?.unwrap_or_else(|| info.sender.clone());

    let fee_bps = msg.fee_bps.unwrap_or(0u64);
    if !validate_protocol_fee(fee_bps) {
        return Err(ContractError::InvalidFee {});
    }

    let config = Config {
        minimum_interval: init_msg.minimum_interval,
        job_registry_contract,
        owner: info.sender.clone(),
        fee_address,
        minimum_amount_per_interval: init_msg.minimum_amount_per_interval,
        max_fee: msg.max_fee.unwrap_or_else(Uint256::zero),
        fee_bps,
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateAgreement {
            receiver,
            amount,
            start_at,
            end_at,
            interval,
        } => try_create_agreement(
            deps, env, info, receiver, amount, start_at, end_at, interval,
        ),
        ExecuteMsg::Transfer { agreement_id } => try_transfer(deps, env, info, agreement_id, None),
        ExecuteMsg::CancelAgreement { agreement_id } => try_cancel(deps, info, agreement_id),
        ExecuteMsg::TerminateAgreement { agreement_id } => try_terminate(deps, env, agreement_id),
        ExecuteMsg::Work { payload } => {
            let payload: WorkPayload = from_binary(&payload).unwrap();
            try_work(deps, env, info, payload.agreement_id)
        }
        ExecuteMsg::UpdateConfig {
            job_registry_contract,
            minimum_interval,
            minimum_amount_per_interval,
            new_owner,
            fee_bps,
            fee_address,
            max_fee,
        } => {
            let api = deps.api;
            update_config(
                deps,
                info,
                optional_addr_validate(api, job_registry_contract)?,
                minimum_interval,
                minimum_amount_per_interval,
                optional_addr_validate(api, new_owner)?,
                fee_bps,
                optional_addr_validate(api, fee_address)?,
                max_fee,
            )
        }
    }
}

#[allow(dead_code)]
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}

#[allow(clippy::too_many_arguments)]
pub fn try_create_agreement(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    receiver: String,
    amount: Uint256,
    start_at: Option<u64>,
    end_at: Option<u64>,
    interval: u64,
) -> Result<Response, ContractError> {
    let receiver_addr = deps.api.addr_validate(&receiver)?;
    if receiver_addr == info.sender {
        return Err(ContractError::CannotSetOwnAccount {});
    }

    let config: Config = CONFIG.load(deps.storage)?;
    if interval < config.minimum_interval {
        return Err(ContractError::InvalidParams {});
    }

    // check if the amount is above minimum amount
    let minimum_amount_per_interval = config.minimum_amount_per_interval;
    if amount < minimum_amount_per_interval {
        return Err(ContractError::InvalidParams {});
    }

    // Starts now or later
    let start_at = start_at.map_or_else(
        || env.block.time,
        |start_at| {
            let s = Timestamp::from_seconds(start_at);
            if s < env.block.time {
                env.block.time
            } else {
                s
            }
        },
    );

    let end_time = end_at.map(Timestamp::from_seconds);

    if let Some(time) = end_time {
        if time <= start_at {
            return Err(ContractError::InvalidEndtime {});
        }
    }

    let interval_due_at = start_at;

    let agreement_id = increment_agreement_id(deps.storage)?;

    let mut messages: Vec<CosmosMsg> = vec![];
    let mut attributes: Vec<Attribute> = vec![
        attr("method", "create_agreement"),
        attr("agreement_id", agreement_id.to_string()),
        attr("from", info.sender.to_string()),
        attr("to", receiver_addr.to_string()),
        attr("amount", amount),
    ];

    let mut agreement = Agreement {
        to: receiver_addr,
        from: info.sender,
        amount,
        created_at: env.block.time,
        interval,
        start_at,
        interval_due_at,
        last_charged: env.block.time,
        end_at: end_time,
    };

    // try charge, skips if no charge
    attempt_charge(&deps, env, &mut agreement, &mut messages, &mut attributes).unwrap_or_default();

    agreements().save(deps.storage, U64Key::from(agreement_id), &agreement)?;

    Ok(Response::new()
        .add_messages(messages)
        .add_attributes(attributes))
}

// Charge & applies state changes
fn attempt_charge(
    deps: &DepsMut,
    env: Env,
    agreement: &mut Agreement,
    messages: &mut Vec<CosmosMsg>,
    attributes: &mut Vec<Attribute>,
) -> Result<(), ContractError> {
    let status = compute_status(agreement, &env.block);
    let has_charge = has_charge(agreement, status, &env.block);

    if !has_charge {
        return Err(ContractError::ZeroTransferableAmount {});
    }

    let config = CONFIG.load(deps.storage)?;

    // Update next due date
    agreement.interval_due_at = agreement.interval_due_at.plus_seconds(agreement.interval);
    agreement.last_charged = env.block.time;

    let mut transfer_amount = agreement.amount;

    let fee_amount = compute_fees(&config, agreement);
    if fee_amount > Uint256::zero() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: agreement.from.clone().into_string(),
            funds: vec![],
            msg: to_binary(&SubWalletExecuteMsg::TransferAToken {
                recipient: config.fee_address.to_string(),
                amount: fee_amount.into(),
            })?,
        }));

        transfer_amount = transfer_amount - fee_amount;
    }

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: agreement.from.clone().into_string(),
        funds: vec![],
        msg: to_binary(&SubWalletExecuteMsg::TransferAToken {
            recipient: agreement.to.clone().to_string(),
            amount: transfer_amount.into(),
        })?,
    }));

    attributes.push(attr("amount", agreement.amount.to_string()));

    Ok(())
}

/// attempts to cancel an agreement given sender and recipient address.
/// Does not refund outstanding balance to receipient since this contract does not lock up capital.
/// Senders are assumed to be doing regular transfers, either by themselves or relying on Suberra Workers.
/// only the user who created the agreement can cancel the agreement
pub fn try_cancel(
    deps: DepsMut,
    info: MessageInfo,
    agreement_id: u64,
) -> Result<Response, ContractError> {
    let agreement = match agreements().may_load(deps.storage, U64Key::from(agreement_id))? {
        Some(agreement) => agreement,
        None => return Err(ContractError::AgreementNotFound {}),
    };

    if info.sender != agreement.from {
        return Err(ContractError::Unauthorized {});
    }

    agreements().remove(deps.storage, U64Key::from(agreement_id))?;

    Ok(Response::new().add_attributes(vec![
        attr("method", "cancel_agreement"),
        attr("agreement_id", agreement_id.to_string()),
    ]))
}

pub fn try_terminate(
    deps: DepsMut,
    env: Env,
    agreement_id: u64,
) -> Result<Response, ContractError> {
    let agreement = match agreements().may_load(deps.storage, U64Key::from(agreement_id))? {
        Some(agreement) => agreement,
        None => return Err(ContractError::AgreementNotFound {}),
    };
    // Anyone can terminate if it's lapsed or expired.
    let status = compute_status(&agreement, &env.block);
    if status == AgreementStatus::Active || status == AgreementStatus::NotStarted {
        return Err(ContractError::CannotTerminateActiveAgreement {});
    }

    agreements().remove(deps.storage, U64Key::from(agreement_id))?;

    Ok(Response::new().add_attributes(vec![
        attr("method", "terminate_agreement"),
        attr("agreement_id", agreement_id.to_string()),
    ]))
}

pub fn try_transfer(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    agreement_id: u64,
    additional_message: Option<CosmosMsg>,
) -> Result<Response, ContractError> {
    let key = U64Key::from(agreement_id);
    let mut agreement = match agreements().may_load(deps.storage, key.clone())? {
        Some(v) => v,
        None => return Err(ContractError::AgreementNotFound {}),
    };

    let mut messages: Vec<CosmosMsg> = vec![];
    let mut attributes: Vec<Attribute> = vec![
        attr("method", "execute_transfer"),
        attr("agreement_id", agreement_id.to_string()),
    ];

    attempt_charge(&deps, env, &mut agreement, &mut messages, &mut attributes)?;

    agreements().save(deps.storage, key, &agreement)?;
    // allow additional messages (eg. for automation receipt)
    if let Some(msg) = additional_message {
        messages.push(msg);
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attributes(attributes))
}

pub fn try_work(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    agreement_id: u64,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;
    if config.job_registry_contract.is_none() {
        return Err(ContractError::NoJobRegistry {});
    };

    let worker = info.sender.to_string();

    try_transfer(
        deps,
        env,
        info,
        agreement_id,
        Some(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.job_registry_contract.unwrap().to_string(),
            funds: vec![],
            msg: to_binary(&JobsRegistryExecuteMsg::WorkReceipt {
                worker_address: worker,
            })?,
        })),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    job_registry_contract: Option<Addr>,
    new_minimum_interval: Option<u64>,
    new_minimum_amount_per_interval: Option<Uint256>,
    new_owner: Option<Addr>,
    new_fee_bps: Option<u64>,
    new_fee_address: Option<Addr>,
    new_max_fee: Option<Uint256>,
) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    let mut attributes: Vec<Attribute> = vec![attr("method", "update_config")];

    // permission check
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(minimum_interval) = new_minimum_interval {
        config.minimum_interval = minimum_interval;
        attributes.push(attr("minimum_interval", minimum_interval.to_string()))
    }

    if let Some(minimum_amount_per_interval) = new_minimum_amount_per_interval {
        config.minimum_amount_per_interval = minimum_amount_per_interval;
        attributes.push(attr(
            "minimum_interval",
            minimum_amount_per_interval.to_string(),
        ))
    }

    if let Some(job_registry_contract) = job_registry_contract {
        config.job_registry_contract = Some(job_registry_contract.clone());
        attributes.push(attr(
            "job_registry_contract",
            job_registry_contract.to_string(),
        ))
    }

    if let Some(new_owner) = new_owner {
        config.owner = new_owner.clone();
        attributes.push(attr("owner", new_owner.to_string()))
    }

    if let Some(new_fee_bps) = new_fee_bps {
        if !validate_protocol_fee(new_fee_bps) {
            return Err(ContractError::InvalidFee {});
        }

        config.fee_bps = new_fee_bps;
        attributes.push(attr("fee_bps", new_fee_bps.to_string()))
    }

    if let Some(new_max_fee) = new_max_fee {
        config.max_fee = new_max_fee;
        attributes.push(attr("max_fee", new_max_fee.to_string()))
    }

    if let Some(new_fee_address) = new_fee_address {
        config.fee_address = new_fee_address.clone();
        attributes.push(attr("fee_address", new_fee_address.to_string()))
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attributes(attributes))
}

/// validates if the protocol fee is valid. Returns true if valid, false if invalid.
fn validate_protocol_fee(protocol_fee: u64) -> bool {
    // protocol fee can be set anything from 0 (0%) to 500 (5%)
    const MAX_FEE_BPS: u64 = 500u64;
    protocol_fee <= MAX_FEE_BPS
}

pub fn compute_status(agreement: &Agreement, block: &BlockInfo) -> AgreementStatus {
    let block_time = block.time;

    // Expiry takes priority
    let is_expired = agreement.end_at.map_or(false, |f| f <= block_time);
    if is_expired {
        return AgreementStatus::Expired;
    }

    // agreement lapsed after missing 1 whole interval
    let lasped_limit = agreement.interval_due_at.plus_seconds(agreement.interval);
    if lasped_limit < block_time {
        return AgreementStatus::Lapsed;
    }

    if agreement.start_at > block_time {
        return AgreementStatus::NotStarted;
    }

    AgreementStatus::Active
}

pub fn compute_fees(config: &Config, agreement: &Agreement) -> Uint256 {
    if config.fee_bps == 0u64 {
        return Uint256::zero();
    }

    let protocol_fee_rate =
        Decimal256::from_ratio(Uint256::from(config.fee_bps), Uint256::from(10000u64));

    let fee_amount = agreement.amount * protocol_fee_rate;

    let max_fee = config.max_fee;
    if fee_amount > max_fee {
        return max_fee;
    }

    fee_amount
}

pub fn has_charge(agreement: &Agreement, status: AgreementStatus, block: &BlockInfo) -> bool {
    let block_time = block.time;
    // No charge if
    // - agreement not started
    // - not due yet
    // - expired
    // - lapsed
    if status != AgreementStatus::Active {
        return false;
    }

    // true if due date
    agreement.interval_due_at <= block_time
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Agreement { agreement_id } => {
            to_binary(&query_agreement(deps, env, agreement_id)?)
        }
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::AgreementsByOwner {
            owner,
            start_after,
            limit,
        } => to_binary(&query_all_agreements_by_owner(
            deps,
            owner,
            start_after,
            limit,
        )?),
        QueryMsg::AgreementsByReceiver {
            receiver,
            start_after,
            limit,
        } => to_binary(&query_all_agreements_by_receiver(
            deps,
            receiver,
            start_after,
            limit,
        )?),
        QueryMsg::AllAgreements { start_after, limit } => {
            to_binary(&query_all_agreements(deps, start_after, limit)?)
        }
        QueryMsg::OverduedAgreements { start_after, limit } => {
            to_binary(&query_overdue_agreements(deps, env, start_after, limit)?)
        }
        QueryMsg::CanWork { payload } => {
            let work_payload: WorkPayload = from_binary(&payload).unwrap();
            to_binary(&query_can_work(deps, env, work_payload.agreement_id)?)
        }
    }
}

/// query_agreement returns an agreement given (from, receiver) addresses
/// Returns None if agreement cannot be found
pub fn query_agreement(deps: Deps, env: Env, agreement_id: u64) -> StdResult<AgreementResponse> {
    let agreement = agreements().load(deps.storage, U64Key::from(agreement_id))?;

    let status = compute_status(&agreement, &env.block);
    let has_charge = has_charge(&agreement, status.clone(), &env.block);

    let charge_amount = if has_charge {
        agreement.amount
    } else {
        Uint256::zero()
    };

    Ok(AgreementResponse::new(&agreement, &status, &charge_amount))
}

/// query_can_work is called by the Worker nodes - they will query intervalically and only perform work when there is a valid work to be done
fn query_can_work(deps: Deps, env: Env, agreement_id: u64) -> StdResult<bool> {
    let agreement = match agreements().may_load(deps.storage, U64Key::from(agreement_id))? {
        Some(v) => v,
        None => return Ok(false),
    };

    let status = compute_status(&agreement, &env.block);
    let has_charge = has_charge(&agreement, status, &env.block);

    Ok(has_charge)
}

fn query_config(deps: Deps) -> StdResult<Config> {
    let config: Config = CONFIG.load(deps.storage)?;

    Ok(config)
}
