use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::response::MsgInstantiateContractResponse;
use crate::state::{read_config, retrieve_address, store_address, store_config};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Attribute, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Reply, ReplyOn,
    Response, StdError, StdResult, SubMsg, WasmMsg,
};

use protobuf::Message;
use suberra_core::msg::{MigrateMsg, SubwalletInstantiateMsg};
use suberra_core::subwallet_factory::{QueryMsg, SubwalletFactoryConfig as Config};

/// A `reply` call code ID of sub-message.
const INSTANTIATE_SUBWALLET_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    store_config(
        deps.storage,
        &Config {
            subwallet_code_id: msg.subwallet_code_id,
            owner: info.sender,
            anchor_market_contract: deps
                .api
                .addr_validate(msg.anchor_market_contract.as_str())?,
            aterra_token_addr: deps.api.addr_validate(msg.aterra_token_addr.as_str())?,
        },
    )?;
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<Empty>, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            new_subwallet_code_id,
            new_owner,
            new_anchor_market_contract,
            new_aterra_token_addr,
        } => update_config(
            deps,
            env,
            info,
            new_owner,
            new_subwallet_code_id,
            new_anchor_market_contract,
            new_aterra_token_addr,
        ),
        ExecuteMsg::CreateAccount {} => execute_create_account(deps, env, info),
    }
}

#[allow(dead_code)]
#[cfg_attr(not(feature = "library"), entry_point)]
/// Used for migration of contract. Returns the default object of type [`Response`].
/// ## Params
/// * **_deps** is the object of type [`DepsMut`].
///
/// * **_env** is the object of type [`Env`].
///
/// * **_msg** is the object of type [`MigrateMsg`].
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}

/// Updates the configs for the general settings
/// * **deps** is the object of [`DepsMut`]
///
///  * **_env** is the object of type [`Env`]
///
///  * **info** is the object of type [`MessageInfo`]
///
///  * **new_owner** is the address of the new owner of type [`Option<String>`]. Will only change if the value is not null.
///
///  * **anchor_market_contract** is the contract address of the Anchor Money Markets contract of type[`Option<String>`]. Will only change if the value is not null.
///
/// * **aterra_token_addr** is the contract address of the Anchor Token Contract of type [`Option<String>`]. Will only change if the value is not null.
pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_owner: Option<String>,
    new_subwallet_code_id: Option<u64>,
    new_anchor_market_contract: Option<String>,
    new_aterra_token_addr: Option<String>,
) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;
    // permission check
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    let mut attributes: Vec<Attribute> = vec![attr("method", "update_config")];

    if let Some(new_subwallet_code_id) = new_subwallet_code_id {
        config.subwallet_code_id = new_subwallet_code_id;
        attributes.push(attr(
            "new_subwallet_code_id",
            new_subwallet_code_id.to_string(),
        ));
    }

    if let Some(new_owner) = new_owner {
        config.owner = deps.api.addr_validate(new_owner.as_str())?;
        attributes.push(attr("new_owner", new_owner));
    }

    if let Some(new_anchor_market_contract) = new_anchor_market_contract {
        config.anchor_market_contract = deps
            .api
            .addr_validate(new_anchor_market_contract.as_str())?;
        attributes.push(attr(
            "new_anchor_market_contract",
            new_anchor_market_contract,
        ));
    }

    if let Some(new_aterra_token_addr) = new_aterra_token_addr {
        config.aterra_token_addr = deps.api.addr_validate(new_aterra_token_addr.as_str())?;
        attributes.push(attr("new_aterra_token_addr", new_aterra_token_addr));

    };

    store_config(deps.storage, &config)?;
    Ok(Response::new().add_attributes(attributes))
}

/// Creates an account for a given address. Function will initialise a new subwallet for the user and add it to the registry.
/// The full process flow for Suberra account creation:
/// 1. User executes [`ExecuteMsg::CreateAccount`]
/// 2. This function checks if there is an existing account. If there is, throw an error of type [`ContractError`]
/// 3. Factory instantiates a new subwallet contract for the given user
/// 4. After the Subwallet is created, the contract calls the [`reply`] function of this factory contract
/// 5. The Account Creation hook retrieves the `contract_address` and stores the mapping between user and the contract address of the subwallet
/// * **deps** is the object of [`DepsMut`]
///
///  * **_info** is the object of type [`MessageInfo`]
pub fn execute_create_account(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;

    let res = retrieve_address(deps.storage, &deps.api.addr_validate(info.sender.as_str())?);

    if res.is_some() {
        return Err(ContractError::ExistingSubwallet {});
    }

    let sub_msg: Vec<SubMsg> = vec![SubMsg {
        id: INSTANTIATE_SUBWALLET_REPLY_ID,
        msg: WasmMsg::Instantiate {
            admin: Some(config.owner.to_string()),
            code_id: config.subwallet_code_id,
            funds: vec![],
            label: "create_subwallet".to_string(),
            msg: to_binary(&SubwalletInstantiateMsg {
                admins: vec![info.sender.to_string()],
                mutable: true,
                stable_denom: "uusd".to_string(),
                owner_address: info.sender.to_string(),
                subwallet_factory_addr: env.contract.address.to_string(),
            })?,
        }
        .into(),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    }];

    // instantiate a new subwallet from the `code_id`
    Ok(Response::new()
        .add_submessages(sub_msg)
        .add_attribute("action", "create_account"))
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
    if msg.id != INSTANTIATE_SUBWALLET_REPLY_ID {
        // should not enter here as there is only one possible reply ID sent from this contract
        return Err(ContractError::InvalidReplyID {});
    }

    let result = msg.result.unwrap();

    // Find instantiate contract event & it's owner
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

    let contract_address = deps.api.addr_validate(res.get_contract_address())?;

    // store the mapping
    let res = store_address(deps.storage, &owner_addr, &contract_address);
    match res {
        Err(_) => Err(ContractError::UnexpectedError {}),
        Ok(_) => Ok(Response::new().add_attributes(vec![
            attr("method", "account_creation"),
            attr("subwallet", contract_address),
            attr("user", owner_addr),
        ])),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::GetSubwalletAddress { owner_address } => {
            to_binary(&query_address(deps, owner_address)?)
        }
    }
}

fn query_config(deps: Deps) -> StdResult<Config> {
    read_config(deps.storage)
}

fn query_address(deps: Deps, address: String) -> StdResult<Option<String>> {
    let res = retrieve_address(deps.storage, &deps.api.addr_validate(&address)?);
    let subwallet_address = res.map(|v| v.to_string());
    Ok(subwallet_address)
}
