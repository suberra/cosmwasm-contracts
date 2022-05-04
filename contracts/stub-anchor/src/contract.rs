use crate::msg::ContractError;
#[cfg(not(feature = "library"))]
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{read_config, store_config, Config};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{attr, entry_point, from_binary, BankMsg, Coin, CosmosMsg, WasmMsg};
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use moneymarket::market::{Cw20HookMsg, EpochStateResponse};

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
        ExecuteMsg::DepositStable {} => deposit(deps, env, info),
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::UpdateConfig { aterra_contract } => {
            update_config(deps, env, info, aterra_contract)
        }
    }
}

pub fn update_config(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    aterra_contract: String,
) -> Result<Response, ContractError> {
    store_config(
        deps.storage,
        &Config {
            aterra_contract: deps.api.addr_validate(&aterra_contract)?,
        },
    )?;
    Ok(Response::new().add_attribute("method", "update_config"))
}

pub fn deposit(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;

    // Check base denom deposit
    let deposit_amount: Uint256 = info
        .funds
        .iter()
        .find(|c| c.denom == "uusd")
        .map(|c| Uint256::from(c.amount))
        .unwrap_or_else(Uint256::zero);
    let epoch_state = query_epoch_state(env);
    let mint_amount = deposit_amount / epoch_state.exchange_rate;

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.aterra_contract.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: info.sender.to_string(),
                amount: mint_amount.into(),
            })?,
        }))
        .add_attributes(vec![
            attr("action", "deposit_stable"),
            attr("depositor", info.sender),
            attr("mint_amount", mint_amount),
            attr("deposit_amount", deposit_amount),
        ]))
}

/// Called when cw20 token is sent via CW20 Send Msg
pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let token_addr = info.sender;
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::RedeemStable {}) => {
            let config: Config = read_config(deps.storage)?;
            if deps.api.addr_validate(token_addr.as_str())? != config.aterra_contract {
                return Err(ContractError::Unauthorized {});
            }

            let cw20_sender_addr = deps.api.addr_validate(&cw20_msg.sender)?;
            redeem_stable(deps, env, cw20_sender_addr, cw20_msg.amount)
        }
        Err(err) => Err(ContractError::Std(err)),
    }
}

fn redeem_stable(
    deps: DepsMut,
    env: Env,
    sender: cosmwasm_std::Addr,
    burn_amount: cosmwasm_std::Uint128,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;

    let epoch_state = query_epoch_state(env);
    let redeem_amount = Uint256::from(burn_amount) * epoch_state.exchange_rate;

    Ok(Response::new()
        .add_messages(vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.aterra_contract.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: burn_amount,
                })?,
            }),
            CosmosMsg::Bank(BankMsg::Send {
                to_address: sender.to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: redeem_amount.into(),
                }],
            }),
        ])
        .add_attributes(vec![
            attr("action", "redeem_stable"),
            attr("burn_amount", burn_amount),
            attr("redeem_amount", redeem_amount),
        ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::EpochState {
            block_height: _,
            distributed_interest: _,
        } => to_binary(&query_epoch_state(env)),
    }
}

pub fn query_epoch_state(env: Env) -> EpochStateResponse {
    let height = env.block.height;
    let base = Uint256::from(11_984_100u128) + Uint256::from(height);

    EpochStateResponse {
        exchange_rate: Decimal256::from_ratio(base, Uint256::from(10_000_000u128)),
        aterra_supply: Uint256::from(5772974502753477u128) + Uint256::from(height),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::from_binary;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn instantiate_and_query() {
        let mut deps = mock_dependencies(&[]);

        let anyone = "anyone";

        // instantiate the contract
        let instantiate_msg = InstantiateMsg {};
        let info = mock_info(&anyone, &[]);
        instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

        let mut env = mock_env();
        // Query exchange rate
        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::EpochState {
                block_height: None,
                distributed_interest: None,
            },
        )
        .unwrap();
        let epoch: EpochStateResponse = from_binary(&res).unwrap();
        assert_eq!(
            epoch,
            EpochStateResponse {
                exchange_rate: Decimal256::from_ratio(
                    Uint256::from(11_984_100u128) + Uint256::from(12_345u128),
                    Uint256::from(10_000_000u128)
                ),
                aterra_supply: Uint256::from(5772974502753477u128) + Uint256::from(12_345u128),
            }
        );

        // Increase block height, exchange rate should increase
        env.block.height += 10;
        let res = query(
            deps.as_ref(),
            env,
            QueryMsg::EpochState {
                block_height: None,
                distributed_interest: None,
            },
        )
        .unwrap();
        let epoch: EpochStateResponse = from_binary(&res).unwrap();
        assert_eq!(
            epoch,
            EpochStateResponse {
                exchange_rate: Decimal256::from_ratio(
                    Uint256::from(11_984_100u128)
                        + Uint256::from(12_345u128)
                        + Uint256::from(10u128),
                    Uint256::from(10_000_000u128)
                ),
                aterra_supply: Uint256::from(5772974502753477u128)
                    + Uint256::from(12_345u128)
                    + Uint256::from(10u128),
            }
        )
    }
}
