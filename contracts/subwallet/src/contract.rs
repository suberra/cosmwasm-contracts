use schemars::JsonSchema;
use std::fmt;
use std::ops::{AddAssign, Sub};

use admin_core::{
    contract::{
        execute_freeze, execute_unfreeze, execute_update_admins,
        instantiate as whitelist_instantiate, query_admin_list, query_owner,
    },
    msg::InstantiateMsg as CW1InitMsg,
    state::ADMIN_CONFIG,
};
use cosmwasm_bignumber::Uint256;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, DistributionMsg, Empty,
    Env, MessageInfo, Order, Response, StakingMsg, StdResult, Uint128, WasmMsg,
};
use cw0::Expiration;
use cw1::CanExecuteResponse;
use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::msg::{
    AllAllowancesResponse, AllPermissionsResponse, AllowanceInfo, ExecuteMsg, PermissionsInfo,
    QueryMsg,
};
use crate::querier::{calculate_aust_amount, get_aust_balance, get_subwallet_factory_config};
use crate::state::{
    deduct_allowance, read_config, store_config, Allowance, Config, Permissions, ALLOWANCES,
    PERMISSIONS,
};
use suberra_core::msg::{MigrateMsg, SubwalletInstantiateMsg};

// version info for migration info
const CONTRACT_NAME: &str = "suberra-subwallet";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: SubwalletInstantiateMsg,
) -> Result<Response, ContractError> {
    let result = whitelist_instantiate(
        deps.branch(),
        env,
        info,
        CW1InitMsg {
            owner: msg.owner_address.clone(),
            admins: msg.admins,
            mutable: msg.mutable,
        },
    )?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    store_config(
        deps.storage,
        &Config {
            owner_addr: deps.api.addr_validate(&msg.owner_address)?,
            stable_denom: msg.stable_denom.clone(),
            whitelist_contracts: vec![],
            subwallet_factory_addr: deps.api.addr_validate(&msg.subwallet_factory_addr)?,
        },
    )?;

    // required attribute for factory to register owner
    Ok(result.add_attribute("owner", msg.owner_address))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<Empty>,
) -> Result<Response<Empty>, ContractError> {
    match msg {
        ExecuteMsg::Execute { msgs } => execute_execute(deps, env, info, msgs),
        ExecuteMsg::Freeze {} => Ok(execute_freeze(deps, env, info)?),
        ExecuteMsg::Unfreeze {} => Ok(execute_unfreeze(deps, env, info)?),
        ExecuteMsg::UpdateAdmins { admins } => Ok(execute_update_admins(deps, env, info, admins)?),
        ExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => execute_increase_allowance(deps, env, info, spender, amount, expires),
        ExecuteMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => execute_decrease_allowance(deps, env, info, spender, amount, expires),
        ExecuteMsg::SetPermissions {
            spender,
            permissions,
        } => execute_set_permissions(deps, env, info, spender, permissions),
        ExecuteMsg::TransferAToken { amount, recipient } => {
            execute_transfer_atoken(deps, env, info, amount, recipient)
        }
    }
}

pub fn execute_execute<T>(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msgs: Vec<CosmosMsg<T>>,
) -> Result<Response<T>, ContractError>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    let cfg = ADMIN_CONFIG.load(deps.storage)?;

    let config = read_config(deps.storage)?;

    // transactions cannot be processed if the sender isnot the owner and contract is frozen
    if config.owner_addr != info.sender && !cfg.mutable {
        return Err(ContractError::Frozen {});
    }

    // if the sender is not an admin or owner, check for the permissions
    if !cfg.is_admin(info.sender.as_ref()) && config.owner_addr != info.sender {
        for msg in &msgs {
            match msg {
                CosmosMsg::Staking(staking_msg) => {
                    let perm = PERMISSIONS.may_load(deps.storage, &info.sender)?;
                    let perm = perm.ok_or(ContractError::NotAllowed {})?;
                    check_staking_permissions(staking_msg, perm)?;
                }
                CosmosMsg::Distribution(distribution_msg) => {
                    let perm = PERMISSIONS.may_load(deps.storage, &info.sender)?;
                    let perm = perm.ok_or(ContractError::NotAllowed {})?;
                    check_distribution_permissions(distribution_msg, perm)?;
                }
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: _,
                    amount,
                }) => {
                    deduct_allowance(deps.storage, env.clone(), &info.sender, amount.clone())?;
                }
                _ => {
                    return Err(ContractError::MessageTypeRejected {});
                }
            }
        }
    }
    // Relay messages
    let res = Response::new()
        .add_messages(msgs)
        .add_attribute("method", "execute")
        .add_attribute("owner", info.sender);
    Ok(res)
}

pub fn check_staking_permissions(
    staking_msg: &StakingMsg,
    permissions: Permissions,
) -> Result<(), ContractError> {
    match staking_msg {
        StakingMsg::Delegate { .. } => {
            if !permissions.delegate {
                return Err(ContractError::DelegatePerm {});
            }
        }
        StakingMsg::Undelegate { .. } => {
            if !permissions.undelegate {
                return Err(ContractError::UnDelegatePerm {});
            }
        }
        StakingMsg::Redelegate { .. } => {
            if !permissions.redelegate {
                return Err(ContractError::ReDelegatePerm {});
            }
        }
        _ => return Err(ContractError::UnsupportedMessage {}),
    }
    Ok(())
}

pub fn check_distribution_permissions(
    distribution_msg: &DistributionMsg,
    permissions: Permissions,
) -> Result<(), ContractError> {
    match distribution_msg {
        DistributionMsg::SetWithdrawAddress { .. } => {
            if !permissions.withdraw {
                return Err(ContractError::WithdrawAddrPerm {});
            }
        }
        DistributionMsg::WithdrawDelegatorReward { .. } => {
            if !permissions.withdraw {
                return Err(ContractError::WithdrawPerm {});
            }
        }
        _ => return Err(ContractError::UnsupportedMessage {}),
    }
    Ok(())
}

/// Increases allowance for a `spender` address.
pub fn execute_increase_allowance<T>(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    spender: String,
    amount: Coin,
    expires: Option<Expiration>,
) -> Result<Response<T>, ContractError>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    check_is_admin_or_owner(deps.as_ref(), info.sender.clone())?;

    let spender_addr = deps.api.addr_validate(&spender)?;
    if info.sender == spender_addr {
        return Err(ContractError::CannotSetOwnAccount {});
    }

    ALLOWANCES.update::<_, ContractError>(deps.storage, &spender_addr, |allow| {
        let prev_expires = allow
            .as_ref()
            .map(|allow| allow.expires)
            .unwrap_or_default();

        let mut allowance = allow
            .filter(|allow| !allow.expires.is_expired(&env.block))
            .unwrap_or_default();

        if let Some(exp) = expires {
            if exp.is_expired(&env.block) {
                return Err(ContractError::SettingExpiredAllowance(exp));
            }

            allowance.expires = exp;
        } else if prev_expires.is_expired(&env.block) {
            return Err(ContractError::SettingExpiredAllowance(prev_expires));
        }

        allowance.balance.add_assign(amount.clone());
        Ok(allowance)
    })?;

    let res = Response::new()
        .add_attribute("action", "increase_allowance")
        .add_attribute("owner", info.sender)
        .add_attribute("spender", spender)
        .add_attribute("denomination", amount.denom)
        .add_attribute("amount", amount.amount);
    Ok(res)
}

pub fn execute_decrease_allowance<T>(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    spender: String,
    amount: Coin,
    expires: Option<Expiration>,
) -> Result<Response<T>, ContractError>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    check_is_admin_or_owner(deps.as_ref(), info.sender.clone())?;

    let spender_addr = deps.api.addr_validate(&spender)?;
    if info.sender == spender_addr {
        return Err(ContractError::CannotSetOwnAccount {});
    }

    let allowance =
        ALLOWANCES.update::<_, ContractError>(deps.storage, &spender_addr, |allow| {
            // Fail fast
            let mut allowance = allow
                .filter(|allow| !allow.expires.is_expired(&env.block))
                .ok_or(ContractError::NoAllowance {})?;

            if let Some(exp) = expires {
                if exp.is_expired(&env.block) {
                    return Err(ContractError::SettingExpiredAllowance(exp));
                }

                allowance.expires = exp;
            }

            allowance.balance = allowance.balance.sub_saturating(amount.clone())?; // Tolerates underflows (amount bigger than balance), but fails if there are no tokens at all for the denom (report potential errors)
            Ok(allowance)
        })?;

    if allowance.balance.is_empty() {
        ALLOWANCES.remove(deps.storage, &spender_addr);
    }

    let res = Response::new()
        .add_attribute("method", "decrease_allowance")
        .add_attribute("owner", info.sender)
        .add_attribute("spender", spender)
        .add_attribute("denomination", amount.denom)
        .add_attribute("amount", amount.amount);
    Ok(res)
}

/// Transfers aUST to a receipient
///
///  * **deps** is the object of [`DepsMut`]
///
///  * **_info** is the object of type [`MessageInfo`]
///
///  * **_env** is the object of type [`Env`]
pub fn execute_transfer_atoken<T>(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    recipient: String,
) -> Result<Response<T>, ContractError>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    let coin = Coin {
        denom: "uusd".to_string(),
        amount,
    };

    let cfg = ADMIN_CONFIG.load(deps.storage)?;

    // transactions cannot be processed if the sender is not the owner and contract is frozen
    if cfg.owner != info.sender && !cfg.mutable {
        return Err(ContractError::Frozen {});
    }

    let factory_config = get_subwallet_factory_config(deps.as_ref())?;

    let market_contract = factory_config.anchor_market_contract;
    let atoken = factory_config.aterra_token_addr;

    let aust_amount: Uint256 = calculate_aust_amount(
        deps.as_ref(),
        market_contract.to_string(),
        Uint256::from(amount.u128()),
    )?;
    let aust_amount_chargeable = Uint128::from(aust_amount);

    let aust_balance: Uint128 = get_aust_balance(
        deps.as_ref(),
        atoken.to_string(),
        env.contract.address.to_string(),
    )?;

    if aust_amount_chargeable > aust_balance {
        return Err(ContractError::InsufficientFunds {});
    }

    // if the user is not the owner, deduct the allowance in UST
    ALLOWANCES.update::<_, ContractError>(deps.storage, &info.sender, |allow| {
        let mut allowance = allow.ok_or(ContractError::NoAllowance {})?;
        if allowance.expires.is_expired(&env.block) {
            return Err(ContractError::NoAllowance {});
        }

        // Decrease allowance
        allowance.balance = allowance.balance.sub(coin.clone())?;
        Ok(allowance)
    })?;

    let aust_amount_u128: u128 = aust_amount.into();

    Ok(Response::new()
        .add_messages(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: atoken.into_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient,
                amount: Uint128::from(aust_amount_u128),
            })?,
        })])
        .add_attributes(vec![attr("method", "transfer_atoken")]))
}

pub fn execute_set_permissions<T>(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    spender: String,
    perm: Permissions,
) -> Result<Response<T>, ContractError>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    check_is_admin_or_owner(deps.as_ref(), info.sender.clone())?;

    let spender_addr = deps.api.addr_validate(&spender)?;
    if info.sender == spender_addr {
        return Err(ContractError::CannotSetOwnAccount {});
    }
    PERMISSIONS.save(deps.storage, &spender_addr, &perm)?;

    let res = Response::new()
        .add_attribute("action", "set_permissions")
        .add_attribute("owner", info.sender)
        .add_attribute("spender", spender)
        .add_attribute("permissions", perm.to_string());
    Ok(res)
}

// checks if the contract is executed by an owner or admin. If it is by an admin, it checks if the contract is frozen
fn check_is_admin_or_owner(deps: Deps, sender: Addr) -> Result<bool, ContractError> {
    let cfg = ADMIN_CONFIG.load(deps.storage).unwrap();
    // function can only be called by the owner or admins
    if !cfg.is_admin(sender.as_ref()) && sender.as_ref() != cfg.owner {
        return Err(ContractError::Unauthorized {});
    }

    if cfg.owner != sender.as_ref() && !cfg.mutable {
        return Err(ContractError::Frozen {});
    }

    Ok(true)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::AdminList {} => to_binary(&query_admin_list(deps)?),
        QueryMsg::Owner {} => to_binary(&query_owner(deps)?),
        QueryMsg::Allowance { spender } => to_binary(&query_allowance(deps, env, spender)?),
        QueryMsg::Permissions { spender } => to_binary(&query_permissions(deps, spender)?),
        QueryMsg::CanExecute { sender, msg } => {
            to_binary(&query_can_execute(deps, env, sender, msg)?)
        }
        QueryMsg::AllAllowances { start_after, limit } => {
            to_binary(&query_all_allowances(deps, env, start_after, limit)?)
        }
        QueryMsg::AllPermissions { start_after, limit } => {
            to_binary(&query_all_permissions(deps, start_after, limit)?)
        }
    }
}

// if the subkey has no allowance, return an empty struct (not an error)
pub fn query_allowance(deps: Deps, env: Env, spender: String) -> StdResult<Allowance> {
    // we can use unchecked here as it is a query - bad value means a miss, we never write it
    let spender = deps.api.addr_validate(&spender)?;
    let allow = ALLOWANCES
        .may_load(deps.storage, &spender)?
        .filter(|allow| !allow.expires.is_expired(&env.block))
        .unwrap_or_default();

    Ok(allow)
}

// if the subkey has no permissions, return an empty struct (not an error)
pub fn query_permissions(deps: Deps, spender: String) -> StdResult<Permissions> {
    let spender = deps.api.addr_validate(&spender)?;
    let permissions = PERMISSIONS
        .may_load(deps.storage, &spender)?
        .unwrap_or_default();
    Ok(permissions)
}

pub fn query_can_execute(
    deps: Deps,
    env: Env,
    sender: String,
    msg: CosmosMsg,
) -> StdResult<CanExecuteResponse> {
    Ok(CanExecuteResponse {
        can_execute: can_execute(deps, env, sender, msg)?,
    })
}

// this can just return booleans and the query_can_execute wrapper creates the struct once, not on every path
fn can_execute(deps: Deps, env: Env, sender: String, msg: CosmosMsg) -> StdResult<bool> {
    let cfg = ADMIN_CONFIG.load(deps.storage)?;
    if cfg.is_owner(&sender) || (cfg.is_admin(&sender) && cfg.mutable) {
        return Ok(true);
    }

    let sender = deps.api.addr_validate(&sender)?;
    match msg {
        CosmosMsg::Bank(BankMsg::Send { amount, .. }) => {
            // now we check if there is enough allowance for this message
            let allowance = ALLOWANCES.may_load(deps.storage, &sender)?;
            match allowance {
                // if there is an allowance, we subtract the requested amount to ensure it is covered (error on underflow)
                Some(allow) => {
                    Ok(!allow.expires.is_expired(&env.block) && allow.balance.sub(amount).is_ok())
                }
                None => Ok(false),
            }
        }
        CosmosMsg::Staking(staking_msg) => {
            let perm_opt = PERMISSIONS.may_load(deps.storage, &sender)?;
            match perm_opt {
                Some(permission) => Ok(check_staking_permissions(&staking_msg, permission).is_ok()),
                None => Ok(false),
            }
        }
        CosmosMsg::Distribution(distribution_msg) => {
            let perm_opt = PERMISSIONS.may_load(deps.storage, &sender)?;
            match perm_opt {
                Some(permission) => {
                    Ok(check_distribution_permissions(&distribution_msg, permission).is_ok())
                }
                None => Ok(false),
            }
        }
        _ => Ok(false),
    }
}

#[allow(dead_code)]
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

fn calc_limit(request: Option<u32>) -> usize {
    request.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize
}

// return a list of all allowances here
pub fn query_all_allowances(
    deps: Deps,
    env: Env,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<AllAllowancesResponse> {
    let limit = calc_limit(limit);
    // we use raw addresses here....
    let start = start_after.map(Bound::exclusive);

    let res: StdResult<Vec<AllowanceInfo>> = ALLOWANCES
        .range(deps.storage, start, None, Order::Ascending)
        .filter(|item| {
            if let Ok((_, allow)) = item {
                !allow.expires.is_expired(&env.block)
            } else {
                true
            }
        })
        .take(limit)
        .map(|item| {
            item.and_then(|(k, allow)| {
                Ok(AllowanceInfo {
                    spender: String::from_utf8(k)?,
                    balance: allow.balance,
                    expires: allow.expires,
                })
            })
        })
        .collect();
    Ok(AllAllowancesResponse { allowances: res? })
}

// return a list of all permissions here
pub fn query_all_permissions(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<AllPermissionsResponse> {
    let limit = calc_limit(limit);
    let start = start_after.map(Bound::exclusive);

    let res: StdResult<Vec<PermissionsInfo>> = PERMISSIONS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            item.and_then(|(k, perm)| {
                Ok(PermissionsInfo {
                    spender: String::from_utf8(k)?,
                    permissions: perm,
                })
            })
        })
        .collect();
    Ok(AllPermissionsResponse { permissions: res? })
}
