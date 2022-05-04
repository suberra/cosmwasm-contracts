use crate::state::read_config;
use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{to_binary, Deps, QueryRequest, StdResult, Uint128, WasmQuery};
use cw20::BalanceResponse;
use cw20::Cw20QueryMsg;
use moneymarket::market::EpochStateResponse;
use moneymarket::market::QueryMsg as MarketQueryMsg;
use suberra_core::subwallet_factory::{
    QueryMsg as SubwalletFactoryQueryMsg, SubwalletFactoryConfig,
};

pub fn calculate_aust_amount(
    deps: Deps,
    moneymarket_address: String,
    amount: Uint256,
) -> StdResult<Uint256> {
    let epoch_state_response: EpochStateResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: moneymarket_address,
            msg: to_binary(&MarketQueryMsg::EpochState {
                block_height: Option::None,
                distributed_interest: Option::None,
            })?,
        }))?;
    let aust_amount = amount / epoch_state_response.exchange_rate;
    Ok(aust_amount)
}

pub fn get_aust_balance(
    deps: Deps,
    aterra_token_addr: String,
    address: String,
) -> StdResult<Uint128> {
    let balance_response: BalanceResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: aterra_token_addr,
            msg: to_binary(&Cw20QueryMsg::Balance { address })?,
        }))?;

    Ok(balance_response.balance)
}

/// A query wrapper to retrieve config from SubwalletFactory
pub fn get_subwallet_factory_config(deps: Deps) -> StdResult<SubwalletFactoryConfig> {
    let config = read_config(deps.storage)?;

    let subwallet_config: SubwalletFactoryConfig =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: config.subwallet_factory_addr.to_string(),
            msg: to_binary(&SubwalletFactoryQueryMsg::Config {})?,
        }))?;

    Ok(subwallet_config)
}
