#[cfg(not(feature = "library"))]
use crate::jobs::query_all_jobs;
use crate::msg::{AllJobsResponse, ConfigResponse, JobInfo};
use crate::querier::deduct_tax;
use crate::state::{Config, Job, CONFIG, COUNT, CREDITS, JOBS};
use cosmwasm_std::{
    entry_point, to_binary, Addr, Api, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Response, StdResult,
};
use cw0::NativeBalance;
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{CountResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use suberra_core::msg::MigrateMsg;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:jobs-registry";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// hard cap of 10 admins to prevent uncapped arrays
const MAXIMUM_ADMIN_LIST_SIZE: usize = 10;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = Config {
        owner: info.sender.clone(),
        admins: vec![],
        base_fee: vec![],
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &state)?;

    COUNT.save(deps.storage, &0)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddJob {
            contract_address,
            name,
        } => {
            let contract_addr = deps.api.addr_validate(&contract_address)?;
            try_add_job(deps, info, contract_addr, name)
        }
        ExecuteMsg::RemoveJob { contract_address } => {
            let contract_addr = deps.api.addr_validate(&contract_address)?;
            try_remove_job(deps, info, contract_addr)
        }
        ExecuteMsg::AddCredits { contract_address } => {
            let contract_addr = deps.api.addr_validate(&contract_address)?;
            try_add_credits(deps, info, contract_addr)
        }
        ExecuteMsg::WorkReceipt { worker_address } => {
            let worker_addr = deps.api.addr_validate(&worker_address)?;
            try_work_receipt(deps, info, worker_addr)
        }
        ExecuteMsg::UpdateAdmins { admins } => try_update_admins(deps, info, admins),
        ExecuteMsg::SetBaseFee { base_fee } => try_set_base_fee(deps, info, base_fee),
    }
}

#[allow(dead_code)]
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}

pub fn try_remove_job(
    deps: DepsMut,
    info: MessageInfo,
    contract_address: Addr,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let job = JOBS.may_load(deps.storage, &contract_address)?;
    match job {
        None => return Err(ContractError::JobNotFound {}),
        Some(j) => {
            if !config.is_owner(&info.sender)
                && !config.is_admin(&info.sender)
                && j.owner != info.sender
            {
                return Err(ContractError::Unauthorized {});
            }

            JOBS.remove(deps.storage, &contract_address);
        }
    };
    Ok(Response::new()
        .add_attribute("method", "try_remove_job")
        .add_attribute("contract", &contract_address))
}

/// try_work_receipt : called when the work is completed by Workers
/// This transfers the credits applicable to the worker_address.
pub fn try_work_receipt(
    deps: DepsMut,
    info: MessageInfo,
    worker_address: Addr,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let base_fee = &config.base_fee;

    let job = JOBS.may_load(deps.storage, &info.sender)?;
    match job {
        None => return Err(ContractError::JobNotFound {}),
        Some(j) if !j.active => return Err(ContractError::JobNotFound {}),
        _ => {}
    };

    let mut fees_after_tax: Vec<Coin> = vec![];
    for coin in base_fee.iter() {
        fees_after_tax.push(deduct_tax(deps.as_ref(), coin.clone())?);
    }

    let mut messages = vec![];
    if !fees_after_tax.is_empty() {
        // if there are fees, update the balance in the Credits
        CREDITS.update(
            deps.storage,
            &info.sender,
            |_balance| -> Result<NativeBalance, ContractError> {
                match _balance {
                    Some(balance) => {
                        let new_balance = balance - base_fee.clone();
                        match new_balance {
                            Ok(bal) => Ok(bal),
                            Err(_) => Err(ContractError::JobInsufficientCredits),
                        }
                    }
                    None => Err(ContractError::JobInsufficientCredits),
                }
            },
        )?;

        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: worker_address.to_string(),
            amount: fees_after_tax,
        }));
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("method", "try_work_receipt")
        .add_attribute("contract", &info.sender)
        .add_attribute("worker", &worker_address)
        .add_attribute(
            "reward",
            &base_fee
                .iter()
                .map(|coin: &Coin| coin.to_string())
                .collect::<Vec<String>>()
                .join(","),
        ))
}

/// try_set_base_fee sets the base fee that can be claimable by the Workers. Once set, jobs must be funded with credits.
pub fn try_set_base_fee(
    deps: DepsMut,
    info: MessageInfo,
    base_fee: Vec<Coin>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if !config.is_owner(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    config.base_fee = base_fee.clone();

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "try_set_base_fee")
        .add_attribute(
            "base_fee",
            base_fee
                .iter()
                .map(|coin: &Coin| coin.to_string())
                .collect::<Vec<String>>()
                .join(","),
        ))
}

/// try_add_job adds a contract to the job-registry. Workers can only claim fees for upkeep on the contracts that have been added to the registry.
pub fn try_add_job(
    deps: DepsMut,
    info: MessageInfo,
    contract_address: Addr,
    name: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if !config.is_owner(&info.sender) && !config.is_admin(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let job = JOBS.may_load(deps.storage, &contract_address)?;
    match job {
        Some(j) if j.active => return Err(ContractError::JobExist {}),
        _ => {}
    };

    let job_id = COUNT.update(deps.storage, |mut count| -> Result<u64, ContractError> {
        count += 1;
        Ok(count)
    })?;

    let job =
        JOBS.key(&contract_address)
            .update(deps.storage, |_state| -> Result<_, ContractError> {
                let job = Job {
                    owner: info.sender,
                    contract: contract_address.clone(),
                    active: true,
                    job_id,
                    name,
                };
                Ok(job)
            })?;

    Ok(Response::new()
        .add_attribute("method", "try_add_job")
        .add_attribute("contract", job.contract)
        .add_attribute("job_id", job.job_id.to_string()))
}

/// add_credits to the contract. Contract needs to have credits so that the workers can be paid
pub fn try_add_credits(
    deps: DepsMut,
    info: MessageInfo,
    contract_address: Addr,
) -> Result<Response, ContractError> {
    let job = JOBS.may_load(deps.storage, &contract_address)?;
    match job {
        Some(_) => {}
        None => return Err(ContractError::JobNotFound {}),
    };

    CREDITS.key(&contract_address).update(
        deps.storage,
        |_balance| -> Result<_, ContractError> {
            let new_balance = match _balance {
                Some(balance) => balance + NativeBalance(info.funds),
                None => NativeBalance(info.funds),
            };
            Ok(new_balance)
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "try_add_credits")
        .add_attribute("contract", contract_address))
}

pub fn try_update_admins(
    deps: DepsMut,
    info: MessageInfo,
    admins: Vec<String>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if !config.is_owner(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    if admins.len() > MAXIMUM_ADMIN_LIST_SIZE {
        return Err(ContractError::InvalidParam {});
    }

    config.admins = map_validate(deps.api, &admins)?;
    CONFIG.save(deps.storage, &config)?;

    let res = Response::new().add_attribute("action", "update_admins");
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
        QueryMsg::NumJobs {} => to_binary(&query_num_jobs(deps)?),
        QueryMsg::GetJobCredits { contract_address } => {
            let contract_addr = deps.api.addr_validate(&contract_address)?;
            to_binary(&query_job_credits(deps, contract_addr)?)
        }
        QueryMsg::GetJob { contract_address } => {
            let contract_addr = deps.api.addr_validate(&contract_address)?;
            to_binary(&query_job(deps, contract_addr)?)
        }
        QueryMsg::AllJobs { start_after, limit } => {
            to_binary(&query_jobs(deps, start_after, limit)?)
        }
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: config.owner.to_string(),
        admins: config.admins.into_iter().map(|a| a.into()).collect(),
        base_fee: config.base_fee,
    })
}

fn query_num_jobs(deps: Deps) -> StdResult<CountResponse> {
    let state = COUNT.load(deps.storage)?;
    Ok(CountResponse { count: state })
}

/// query_job_credits returns the amount of available credits for a given contract address
fn query_job_credits(deps: Deps, contract_address: Addr) -> StdResult<NativeBalance> {
    let credits = CREDITS.load(deps.storage, &contract_address)?;
    Ok(credits)
}

fn query_job(deps: Deps, contract_address: Addr) -> StdResult<JobInfo> {
    let job = JOBS.load(deps.storage, &contract_address)?;
    Ok(JobInfo::from(job))
}

fn query_jobs(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<AllJobsResponse> {
    query_all_jobs(deps, start_after, limit)
}

pub fn map_validate(api: &dyn Api, admins: &[String]) -> StdResult<Vec<Addr>> {
    admins.iter().map(|addr| api.addr_validate(addr)).collect()
}
