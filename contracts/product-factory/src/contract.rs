use std::convert::TryInto;

use crate::error::ContractError;
use crate::response::MsgInstantiateContractResponse;
use crate::state::{increment_product_id, Config, CONFIG, PRODUCTS};
use cosmwasm_bignumber::Uint256;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Addr, Attribute, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order,
    Reply, ReplyOn, Response, StdError, StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use cw_storage_plus::{Bound, U64Key};
use protobuf::Message;
use suberra_core::msg::{JobsRegistryExecuteMsg, MigrateMsg, ProductInstantiateMsg};
use suberra_core::product_factory::{
    ConfigResponse, CreateProductExecuteMsg, ExecuteMsg, InstantiateMsg, ProductsResponse, QueryMsg,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:product-factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
/// A `reply` call code ID of sub-message.
const INSTANTIATE_PRODUCT_REPLY_ID: u64 = 1;

/// pagination limits
const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if !validate_protocol_fee(msg.protocol_fee_bps) {
        return Err(ContractError::InvalidParam {});
    }

    let config = Config {
        owner: info.sender.clone(),
        product_code_id: msg.product_code_id,
        protocol_fee_bps: msg.protocol_fee_bps,
        min_protocol_fee: msg.min_protocol_fee,
        min_amount_per_interval: msg.min_amount_per_interval,
        fee_address: deps.api.addr_validate(&msg.fee_address)?,
        job_registry_address: deps.api.addr_validate(&msg.job_registry_address)?,
    };

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
        ExecuteMsg::CreateProduct { product_info } => {
            execute_create_product(deps, env, info, product_info)
        }
        ExecuteMsg::UpdateConfig {
            new_owner,
            new_product_code_id,
            new_protocol_fee_bps,
            new_min_protocol_fee,
            new_min_amount_per_interval,
            new_fee_address,
            new_job_registry_address,
        } => update_config(
            deps,
            env,
            info,
            new_owner,
            new_product_code_id,
            new_protocol_fee_bps,
            new_min_protocol_fee,
            new_min_amount_per_interval,
            new_fee_address,
            new_job_registry_address,
        ),
    }
}

/// # Description
/// Updates the generate settings. Accepts optional values and only updates value if it is provided.
///
/// ## Executor
/// Only owner can execute this function
#[allow(clippy::too_many_arguments)]
pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_owner: Option<String>,
    new_product_code_id: Option<u64>,
    new_protocol_fee_bps: Option<u64>,
    new_min_protocol_fee: Option<Uint256>,
    new_min_amount_per_interval: Option<Uint256>,
    new_fee_address: Option<String>,
    new_job_registry_address: Option<String>,
) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;
    // permission check
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    let mut attributes: Vec<Attribute> = vec![attr("method", "update_config")];

    if let Some(new_owner) = new_owner {
        config.owner = deps.api.addr_validate(new_owner.as_str())?;
        attributes.push(attr("new_owner", new_owner));
    }

    if let Some(new_protocol_fee_bps) = new_protocol_fee_bps {
        if !validate_protocol_fee(new_protocol_fee_bps) {
            return Err(ContractError::InvalidParam {});
        }

        config.protocol_fee_bps = new_protocol_fee_bps;
        attributes.push(attr(
            "new_protocol_fee_bps",
            new_protocol_fee_bps.to_string(),
        ));
    }

    if let Some(new_min_protocol_fee) = new_min_protocol_fee {
        config.min_protocol_fee = new_min_protocol_fee;
        attributes.push(attr(
            "new_min_protocol_fee",
            new_min_protocol_fee.to_string(),
        ));
    }

    if let Some(new_min_amount_per_interval) = new_min_amount_per_interval {
        config.min_amount_per_interval = new_min_amount_per_interval;
        attributes.push(attr(
            "min_amount_per_interval",
            new_min_amount_per_interval.to_string(),
        ));
    }

    if let Some(new_product_code_id) = new_product_code_id {
        config.product_code_id = new_product_code_id;
        attributes.push(attr("new_product_code_id", new_product_code_id.to_string()));
    }

    if let Some(new_fee_address) = new_fee_address {
        config.fee_address = deps.api.addr_validate(new_fee_address.as_str())?;
        attributes.push(attr("new_fee_address", new_fee_address));
    }

    if let Some(new_job_registry_address) = new_job_registry_address {
        config.job_registry_address = deps.api.addr_validate(new_job_registry_address.as_str())?;
        attributes.push(attr("new_job_registry_address", new_job_registry_address));
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attributes(attributes))
}

/// # Description
/// Creates a product by first instantiating a product subscription contract.
/// Returns the [`Response`] with the specified attributes if the operation was successful, or a [`ContractError`] if the contract was not created
/// ## Params
/// * **deps** is the object of type [`DepsMut`].
///
/// * **_env** is the object of type [`Env`]
///
/// * **param** is the object of type [`CreateProductExecuteMsg`]
pub fn execute_create_product(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    param: CreateProductExecuteMsg,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    if param.unit_amount < config.min_amount_per_interval {
        return Err(ContractError::InvalidParam {});
    }

    let product: ProductInstantiateMsg = ProductInstantiateMsg {
        receiver_address: param.receiver_address,
        unit_amount: param.unit_amount,
        initial_amount: param.initial_amount,
        unit_interval_hour: param.unit_interval_hour,
        max_amount_chargeable: param.max_amount_chargeable,
        additional_grace_period_hour: param.additional_grace_period_hour,
        uri: param.uri,
        owner: info.sender.to_string(),
        admins: param.admins,
        mutable: param.mutable,
        factory_address: env.contract.address.to_string(),
    };

    let sub_msg: Vec<SubMsg> = vec![SubMsg {
        id: INSTANTIATE_PRODUCT_REPLY_ID,
        msg: WasmMsg::Instantiate {
            admin: Some(config.owner.to_string()),
            code_id: config.product_code_id,
            funds: vec![],
            label: "create_product".to_string(),
            msg: to_binary(&product)?,
        }
        .into(),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    }];

    Ok(Response::new()
        .add_submessages(sub_msg)
        .add_attribute("action", "create_product"))
}

/// validates if the protocol fee is valid. Returns true if valid, false if invalid.
fn validate_protocol_fee(protocol_fee: u64) -> bool {
    // protocol fee can be set anything from 0 (0%) to 500 (5%). Protocol deliberately caps the maximum configurable % fee to be 5%
    const MAX_FEE: u64 = 500u64;
    protocol_fee <= MAX_FEE
}

/// # Description
/// The entry point to the contract for processing the reply from the submessage.
/// # Params
/// * **deps** is the object of type [`DepsMut`].
///
/// * **_env** is the object of type [`Env`].
///
/// * **msg** is the object of type [`Reply`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.id != INSTANTIATE_PRODUCT_REPLY_ID {
        // should not enter here as there is only one possible reply ID sent from this contract
        return Err(ContractError::InvalidReplyID {});
    }

    let result = msg.result.unwrap();

    // Find instantiate subscription product tx event
    let wasm = result.events.iter().find(|&e| e.ty == "wasm");
    let wasm = wasm.unwrap();
    let owner = &wasm
        .attributes
        .iter()
        .find(|&attr| attr.key == "owner")
        .unwrap()
        .value;
    let owner_addr = deps.api.addr_validate(owner)?;

    let data = result.data.unwrap();
    let res: MsgInstantiateContractResponse =
        Message::parse_from_bytes(data.as_slice()).map_err(|_| {
            StdError::parse_err("MsgInstantiateContractResponse", "failed to parse data")
        })?;

    let product_id: u64 = increment_product_id(deps.storage)?;
    let contract_address = res.get_contract_address();

    // store the mapping
    PRODUCTS.save(
        deps.storage,
        (owner_addr, U64Key::from(product_id)),
        &deps.api.addr_validate(contract_address)?,
    )?;

    // Adds the newly created contract to the job registry contract
    let config: Config = CONFIG.load(deps.storage)?;
    let add_job_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.job_registry_address.to_string(),
        funds: vec![],
        msg: to_binary(&JobsRegistryExecuteMsg::AddJob {
            contract_address: contract_address.to_string(),
            name: product_id.to_string(),
        })?,
    });

    Ok(Response::new()
        .add_messages(vec![add_job_msg])
        .add_attributes(vec![
            attr("action", "register"),
            attr("product_id", product_id.to_string()),
            attr("contract", contract_address),
        ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::ProductsByOwner {
            owner,
            start_after,
            limit,
        } => to_binary(&query_products_by_owner(deps, owner, start_after, limit)?),
    }
}

fn query_products_by_owner(
    deps: Deps,
    owner: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ProductsResponse> {
    let owner = deps.api.addr_validate(&owner)?;

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(U64Key::from).map(Bound::exclusive);

    let mut last_key: u64 = 0u64;

    let products: StdResult<Vec<Addr>> = PRODUCTS
        .prefix(owner)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|elem| {
            let (k, contract_addr) = elem.unwrap();
            let id = u64::from_be_bytes(k[..].try_into().expect("unexpected key length"));
            last_key = id;
            Ok(contract_addr)
        })
        .collect();

    Ok(ProductsResponse {
        products: products?,
        last_key: if last_key > 0 { Some(last_key) } else { None },
    })
}

/// ## Description
/// Returns the configs in custom [`ConfigResponse`] structure.
///
/// ### Params
/// * **deps** is the object of type [`Deps`].
fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config: Config = CONFIG.load(deps.storage)?;
    let resp = ConfigResponse {
        owner: config.owner.to_string(),
        product_code_id: config.product_code_id,
        protocol_fee_bps: config.protocol_fee_bps,
        min_protocol_fee: config.min_protocol_fee,
        fee_address: config.fee_address.to_string(),
        job_registry_address: config.job_registry_address.to_string(),
    };
    Ok(resp)
}

/// Used for migration of contract. Returns the default object of type [`Response`].
/// ## Params
/// * **_deps** is the object of type [`Deps`].
///
/// * **_env** is the object of type [`Env`].
///
/// * **_msg** is the object of type [`MigrateMsg`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
