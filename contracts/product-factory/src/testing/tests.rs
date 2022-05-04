use crate::contract;
use crate::contract::instantiate;
use crate::error::ContractError;
use cosmwasm_bignumber::Uint256;
use cosmwasm_std::attr;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use suberra_core::product_factory::{CreateProductExecuteMsg, ExecuteMsg, InstantiateMsg};

#[test]
fn update_config() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        product_code_id: 1u64,
        protocol_fee_bps: 100,
        min_amount_per_interval: Uint256::from(100u64),
        min_protocol_fee: Uint256::zero(),
        min_unit_interval_hour: 168, // one week
        fee_address: "owner".to_string(),
        job_registry_address: "jobs".to_string(),
    };

    let info = mock_info("deployer", &[]);
    let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());

    // alice attempts to create product
    let product = CreateProductExecuteMsg {
        receiver_address: "receiver".to_string(),
        additional_grace_period_hour: None,
        unit_amount: Uint256::from(123u128),
        initial_amount: Uint256::from(123u128),
        unit_interval_hour: 2592000u64,
        max_amount_chargeable: Some(Uint256::from(123u128)),
        admins: Vec::new(),
        mutable: false,
        uri: "{\"image_url\": \"www.google.com\" }".to_string(),
    };
    let msg = ExecuteMsg::CreateProduct {
        product_info: product,
    };

    // unauthorised user attempts to create product. Should fail
    let alice_info = mock_info("alice", &[]);

    let res = contract::execute(deps.as_mut(), mock_env(), alice_info.clone(), msg.clone());
    match res {
        Err(ContractError::Unauthorized {}) => {}
        _ => panic!("Contract should return an unauthorised error amount"),
    }

    // update config to release the restrictions
    let update_config_msg = ExecuteMsg::UpdateConfig {
        new_owner: None,
        new_fee_address: None,
        new_job_registry_address: None,
        new_min_amount_per_interval: None,
        new_min_protocol_fee: None,
        new_min_unit_interval_hour: None,
        new_product_code_id: None,
        new_protocol_fee_bps: None,
        new_is_restricted: Some(false),
    };
    let _res =
        contract::execute(deps.as_mut(), mock_env(), info, update_config_msg.clone()).unwrap();

    let res =
        contract::execute(deps.as_mut(), mock_env(), alice_info.clone(), msg.clone()).unwrap();
    assert_eq!(res.attributes, vec![attr("action", "create_product")]);
}
