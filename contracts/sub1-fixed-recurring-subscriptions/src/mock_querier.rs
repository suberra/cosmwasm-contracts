use cosmwasm_bignumber::Uint256;
use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Addr, Coin, OwnedDeps, Querier, QuerierResult,
    QueryRequest, SystemError, SystemResult, WasmQuery,
};
use suberra_core::product_factory::{ConfigResponse, ProductsResponse, QueryMsg};
use terra_cosmwasm::TerraQueryWrapper;
/// mock_dependencies is a drop-in replacement for cosmwasm_std::testing::mock_dependencies
/// this uses our CustomQuerier.
pub fn mock_dependencies(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier: WasmMockQuerier =
        WasmMockQuerier::new(MockQuerier::new(&[(MOCK_CONTRACT_ADDR, contract_balance)]));

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier<TerraQueryWrapper>,
    fee_querier: FeeQuerier,
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier<TerraQueryWrapper>) -> Self {
        WasmMockQuerier {
            base,
            fee_querier: FeeQuerier::default(),
        }
    }

    // confiure new fee querier
    pub fn with_fee(
        &mut self,
        protocol_fee_bps: u64,
        min_protocol_fee: Uint256,
        min_amount_per_interval: Uint256,
        min_unit_interval_hour: u64,
    ) {
        self.fee_querier = FeeQuerier::new(
            "owner".to_string(),
            true,
            1,
            protocol_fee_bps,
            min_protocol_fee,
            min_amount_per_interval,
            min_unit_interval_hour,
            "fee_address".to_string(),
            "job_registry".to_string(),
        );
    }
}

#[derive(Clone, Default)]
pub struct FeeQuerier {
    owner: String,
    is_restricted: bool,
    product_code_id: u64,
    protocol_fee_bps: u64,
    min_protocol_fee: Uint256,
    min_amount_per_interval: Uint256,
    min_unit_interval_hour: u64,
    fee_address: String,
    job_registry_address: String,
}

impl FeeQuerier {
    pub fn new(
        owner: String,
        is_restricted: bool,
        product_code_id: u64,
        protocol_fee_bps: u64,
        min_protocol_fee: Uint256,
        min_amount_per_interval: Uint256,
        min_unit_interval_hour: u64,
        fee_address: String,
        job_registry_address: String,
    ) -> Self {
        FeeQuerier {
            owner,
            is_restricted,
            product_code_id,
            protocol_fee_bps,
            min_protocol_fee,
            min_amount_per_interval,
            min_unit_interval_hour,
            fee_address,
            job_registry_address,
        }
    }
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<TerraQueryWrapper> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<TerraQueryWrapper>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart {contract_addr: _, msg})// => {
                => match from_binary(&msg).unwrap() {
                    QueryMsg::Config {} => {
                        let config = ConfigResponse {
                            owner: self.fee_querier.owner.clone(),
                            is_restricted: self.fee_querier.is_restricted,
                            product_code_id: self.fee_querier.product_code_id,
                            protocol_fee_bps:  self.fee_querier.protocol_fee_bps,
                            min_protocol_fee: self.fee_querier.min_protocol_fee,
                            min_amount_per_interval: self.fee_querier.min_amount_per_interval,
                            min_unit_interval_hour: self.fee_querier.min_unit_interval_hour,
                            fee_address: self.fee_querier.fee_address.clone(),
                            job_registry_address: self.fee_querier.job_registry_address.clone(),
                        };
                        SystemResult::Ok(to_binary(&config).into())
                    }
                    QueryMsg::ProductsByOwner { owner: _, start_after: _, limit: _ } => SystemResult::Ok(to_binary(&ProductsResponse{
                        products: vec![Addr::unchecked("addr001")],
                        last_key: None
                    }).into()),
            }
            _ => self.base.handle_query(request),
        }
    }
}
