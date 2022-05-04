use crate::state::read_config;
use cosmwasm_std::{to_binary, Addr, Deps, QueryRequest, StdResult, WasmQuery};
use suberra_core::product_factory::{ConfigResponse, QueryMsg};

/// Retrieves the job registry contract address from the factory
/// ## Params
/// * **deps** is the object of type [`Deps`].
pub fn get_job_registry(deps: Deps) -> StdResult<Addr> {
    let config = read_config(deps.storage)?;

    let product_config: ConfigResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: config.factory_address.to_string(),
            msg: to_binary(&QueryMsg::Config {})?,
        }))?;

    deps.api.addr_validate(&product_config.job_registry_address)
}
