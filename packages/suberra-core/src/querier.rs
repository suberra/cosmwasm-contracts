use crate::product_factory::{
    ConfigResponse as ProductFactoryConfigResponse, QueryMsg as FactoryQueryMsg,
};
use cosmwasm_std::{to_binary, Addr, QuerierWrapper, QueryRequest, StdResult, WasmQuery};

pub fn query_product_factory_config(
    querier: &QuerierWrapper,
    factory_contract: Addr,
) -> StdResult<ProductFactoryConfigResponse> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: factory_contract.to_string(),
        msg: to_binary(&FactoryQueryMsg::Config {})?,
    }))
}
