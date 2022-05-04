use crate::contract;
use crate::error::ContractError;
use crate::msg::{AgreementResponse, AgreementsResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{AgreementStatus, Config};
use cosmwasm_bignumber::Uint256;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{
    attr, coins, from_binary, to_binary, Addr, CosmosMsg, SubMsg, Timestamp, Uint128, WasmMsg,
};
use suberra_core::msg::SubWalletExecuteMsg;

const HOUR_SECONDS: u64 = 3600u64;
const DAY_SECONDS: u64 = 86400u64;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        job_registry_contract: Some("job_registry".to_string()),
        minimum_interval: HOUR_SECONDS,
        minimum_amount_per_interval: Uint256::from(1_000_000u128),
        fee_bps: None,
        fee_address: None,
        max_fee: None,
    };
    let info = mock_info("creator", &coins(1000, "earth"));

    // we can just call .unwrap() to assert this was a success
    let res = contract::instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

// User should be able to create a simple transfer and query should work
#[test]
fn create_agreement() {
    let mut deps = mock_dependencies(&coins(2, "token"));
    let mut env = mock_env();
    // set to 1 January 2021 00:00:00 GMT
    let start_time = 1609459200u64;
    env.block.time = Timestamp::from_seconds(start_time);

    let msg = InstantiateMsg {
        job_registry_contract: Some("job_registry".to_string()),
        minimum_interval: HOUR_SECONDS,
        minimum_amount_per_interval: Uint256::from(1_000_000u128),
        fee_bps: None,
        fee_address: None,
        max_fee: None,
    };
    let info = mock_info("creator", &coins(2, "token"));
    contract::instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // #1. anyone can create a transfer to another address
    let info = mock_info("alice", &coins(2, "token"));
    let msg = ExecuteMsg::CreateAgreement {
        receiver: String::from("bob"),
        amount: Uint256::from(1000000u128),
        start_at: None,
        end_at: None,
        interval: DAY_SECONDS,
    };

    let res = contract::execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Charges first payment immediately
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: String::from("alice"),
            funds: vec![],
            msg: to_binary(&SubWalletExecuteMsg::TransferAToken {
                recipient: String::from("bob"),
                amount: Uint128::from(1000000u128),
            })
            .unwrap(),
        }))]
    );

    let msg = QueryMsg::Agreement { agreement_id: 1 };
    let res = contract::query(deps.as_ref(), env.clone(), msg).unwrap();
    let val: AgreementResponse = from_binary(&res).unwrap();
    // No pending charge after initial transaction
    assert_eq!(
        val,
        AgreementResponse {
            amount: Uint256::from(1000000u128),
            created_at: start_time,
            interval_due_at: start_time + DAY_SECONDS,
            interval: DAY_SECONDS,
            to: Addr::unchecked("bob"),
            from: Addr::unchecked("alice"),
            start_at: start_time,
            end_at: None,
            status: AgreementStatus::Active,
            pending_charge: Uint256::zero(),
            last_charged: start_time
        }
    );

    // Cannot create agreements with below min interval
    let info = mock_info("alice", &coins(2, "token"));
    let msg = ExecuteMsg::CreateAgreement {
        receiver: String::from("bob"),
        amount: Uint256::from(1000000u128),
        start_at: None,
        end_at: None,
        interval: 360,
    };

    let res = contract::execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
    assert_eq!(res, ContractError::InvalidParams {});

    // Cannot create agreements with end_at <= start_at
    let info = mock_info("alice", &coins(2, "token"));
    let msg = ExecuteMsg::CreateAgreement {
        receiver: String::from("bob"),
        amount: Uint256::from(1000000u128),
        start_at: Some(start_time + 1000),
        end_at: Some(start_time + 1000),
        interval: 3600,
    };

    let res = contract::execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
    assert_eq!(res, ContractError::InvalidEndtime {});

    // #2. can create a transfer starting at a later date
    let info = mock_info("alice", &coins(2, "token"));
    let msg = ExecuteMsg::CreateAgreement {
        receiver: String::from("bob"),
        amount: Uint256::from(1000000u128),
        start_at: Some(start_time + 1000),
        end_at: None,
        interval: DAY_SECONDS,
    };

    let res = contract::execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // No initial payment
    assert_eq!(res.messages, vec![]);

    let msg = QueryMsg::Agreement { agreement_id: 2 };
    let res = contract::query(deps.as_ref(), env.clone(), msg).unwrap();
    let val: AgreementResponse = from_binary(&res).unwrap();

    // No pending charge
    assert_eq!(
        val,
        AgreementResponse {
            amount: Uint256::from(1000000u128),
            created_at: start_time,
            interval_due_at: start_time + 1000,
            interval: DAY_SECONDS,
            to: Addr::unchecked("bob"),
            from: Addr::unchecked("alice"),
            start_at: start_time + 1000,
            end_at: None,
            status: AgreementStatus::NotStarted,
            pending_charge: Uint256::zero(),
            last_charged: start_time
        }
    );
}

/// creates an agreement and transfer the amount when bills are due
#[test]
fn create_and_transfer() {
    let mut deps = mock_dependencies(&coins(2, "token"));
    let mut env = mock_env();

    let start_time = 1609459200u64;
    env.block.time = Timestamp::from_seconds(start_time);

    let msg = InstantiateMsg {
        job_registry_contract: Some("job_registry".to_string()),
        minimum_interval: HOUR_SECONDS,
        minimum_amount_per_interval: Uint256::from(1_000_000u128),
        fee_bps: None,
        fee_address: None,
        max_fee: None,
    };
    let info = mock_info("creator", &coins(2, "token"));
    contract::instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // attempts to create an agreement that has lesser than the minimum amount
    let info = mock_info("alice", &coins(2, "token"));
    let msg = ExecuteMsg::CreateAgreement {
        receiver: String::from("bob"),
        amount: Uint256::from(100_000u128),
        start_at: None,
        end_at: None,
        interval: HOUR_SECONDS,
    };
    let res = contract::execute(deps.as_mut(), env.clone(), info, msg);
    match res {
        Err(ContractError::InvalidParams {}) => {}
        _ => panic!(
            "Contract should return an invalid param amount as the amount is lesser than minimum"
        ),
    }

    // Sends $1/hour
    let info = mock_info("alice", &coins(2, "token"));
    let msg = ExecuteMsg::CreateAgreement {
        receiver: String::from("bob"),
        amount: Uint256::from(1_000_000u128),
        start_at: None,
        end_at: None,
        interval: HOUR_SECONDS,
    };
    let _res = contract::execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Unable to transfer immediately as funds are already charged
    let msg = ExecuteMsg::Transfer { agreement_id: 1u64 };
    let info = mock_info("alice", &coins(2, "token"));
    let res = contract::execute(deps.as_mut(), env.clone(), info, msg.clone());
    match res {
        Err(ContractError::ZeroTransferableAmount {}) => {}
        _ => panic!("Contract should return a zero transferable amount"),
    }

    // fast-forward 1 hour and do a transfer
    env.block.time = Timestamp::from_seconds(start_time + HOUR_SECONDS);
    let info = mock_info("alice", &coins(2, "token"));
    let res = contract::execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("method", "execute_transfer"),
            attr("agreement_id", "1"),
            attr("amount", "1000000")
        ]
    );

    // ensure next interval_due & last_charge is updated
    let msg = QueryMsg::Agreement { agreement_id: 1u64 };
    let res = contract::query(deps.as_ref(), env.clone(), msg).unwrap();
    let val: AgreementResponse = from_binary(&res).unwrap();

    assert_eq!(
        val,
        AgreementResponse {
            amount: Uint256::from(1000000u128),
            created_at: start_time,
            interval_due_at: start_time + 2 * HOUR_SECONDS,
            interval: HOUR_SECONDS,
            to: Addr::unchecked("bob"),
            from: Addr::unchecked("alice"),
            start_at: start_time,
            end_at: None,
            status: AgreementStatus::Active,
            pending_charge: Uint256::zero(),
            last_charged: start_time + HOUR_SECONDS
        }
    );
}

// Fees to be charged
#[test]
fn create_with_fees() {
    let mut deps = mock_dependencies(&coins(2, "token"));
    let mut env = mock_env();

    let start_time = 1609459200u64;
    env.block.time = Timestamp::from_seconds(start_time);

    let msg = InstantiateMsg {
        job_registry_contract: Some("job_registry".to_string()),
        minimum_interval: HOUR_SECONDS,
        minimum_amount_per_interval: Uint256::from(1_000_000u128),
        fee_bps: Some(500u64), // 5% fee
        fee_address: None,
        max_fee: Some(Uint256::from(1_000_000u128)), // 1 UST
    };

    let info = mock_info("creator", &coins(2, "token"));
    contract::instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Sends $100/hour
    let info = mock_info("alice", &coins(2, "token"));
    let msg = ExecuteMsg::CreateAgreement {
        receiver: String::from("bob"),
        amount: Uint256::from(100_000_000u128),
        start_at: None,
        end_at: None,
        interval: HOUR_SECONDS,
    };
    let res = contract::execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Should send capped fees of 1 UST to default fee address (owner)
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("alice"),
                funds: vec![],
                msg: to_binary(&SubWalletExecuteMsg::TransferAToken {
                    recipient: String::from("creator"),
                    amount: Uint128::from(1_000_000u128),
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("alice"),
                funds: vec![],
                msg: to_binary(&SubWalletExecuteMsg::TransferAToken {
                    recipient: String::from("bob"),
                    amount: Uint128::from(99_000_000u128),
                })
                .unwrap(),
            }))
        ]
    );

    env.block.time = Timestamp::from_seconds(start_time + HOUR_SECONDS);
    // fast-forward 1 hour and do a transfer

    let msg = ExecuteMsg::Transfer { agreement_id: 1u64 };
    let info = mock_info("alice", &coins(2, "token"));
    let res = contract::execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("method", "execute_transfer"),
            attr("agreement_id", "1"),
            attr("amount", "100000000")
        ]
    );

    // Should still send fees on subsequent transfers
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("alice"),
                funds: vec![],
                msg: to_binary(&SubWalletExecuteMsg::TransferAToken {
                    recipient: String::from("creator"),
                    amount: Uint128::from(1_000_000u128),
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("alice"),
                funds: vec![],
                msg: to_binary(&SubWalletExecuteMsg::TransferAToken {
                    recipient: String::from("bob"),
                    amount: Uint128::from(99_000_000u128),
                })
                .unwrap(),
            }))
        ]
    );

    // Sends $10/hour
    let info = mock_info("alice", &coins(2, "token"));
    let msg = ExecuteMsg::CreateAgreement {
        receiver: String::from("bob"),
        amount: Uint256::from(10_000_000u128),
        start_at: None,
        end_at: None,
        interval: HOUR_SECONDS,
    };
    let res = contract::execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Should send 5% fees of 0.5 UST to default fee address (owner)
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("alice"),
                funds: vec![],
                msg: to_binary(&SubWalletExecuteMsg::TransferAToken {
                    recipient: String::from("creator"),
                    amount: Uint128::from(500_000u128),
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("alice"),
                funds: vec![],
                msg: to_binary(&SubWalletExecuteMsg::TransferAToken {
                    recipient: String::from("bob"),
                    amount: Uint128::from(9_500_000u128),
                })
                .unwrap(),
            }))
        ]
    );
}

#[test]
fn create_and_cancel() {
    let mut deps = mock_dependencies(&coins(2, "token"));
    let mut env = mock_env();

    let start_time = 1609459200u64;
    env.block.time = Timestamp::from_seconds(start_time);

    let msg = InstantiateMsg {
        job_registry_contract: Some("job_registry".to_string()),
        minimum_interval: HOUR_SECONDS,
        minimum_amount_per_interval: Uint256::from(1_000_000u128),
        fee_bps: None,
        fee_address: None,
        max_fee: None,
    };
    let info = mock_info("creator", &coins(2, "token"));
    contract::instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Sends $1/Day
    let info = mock_info("alice", &coins(2, "token"));
    let msg = ExecuteMsg::CreateAgreement {
        receiver: String::from("bob"),
        amount: Uint256::from(1000000u128),
        start_at: None,
        end_at: None,
        interval: DAY_SECONDS,
    };
    contract::execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // cancel agreement immediately
    let info = mock_info("alice", &coins(2, "token"));
    let msg = ExecuteMsg::CancelAgreement { agreement_id: 1u64 };
    let res = contract::execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("method", "cancel_agreement"),
            attr("agreement_id", "1"),
        ]
    );

    env.block.time = Timestamp::from_seconds(start_time + DAY_SECONDS);

    // Cannot transfer after cancellation
    let msg = ExecuteMsg::Transfer { agreement_id: 1u64 };
    let info = mock_info("alice", &coins(2, "token"));
    let res = contract::execute(deps.as_mut(), env.clone(), info, msg.clone());
    match res {
        Err(ContractError::AgreementNotFound {}) => {}
        _ => panic!("Contract should return a zero transferable amount"),
    }
}

#[test]
fn test_multiple_agreements() {
    let mut deps = mock_dependencies(&coins(2, "token"));
    let mut env = mock_env();

    let start_time = 1609459200u64;
    env.block.time = Timestamp::from_seconds(start_time);

    let msg = InstantiateMsg {
        job_registry_contract: Some("job_registry".to_string()),
        minimum_interval: HOUR_SECONDS,
        minimum_amount_per_interval: Uint256::from(1_000_000u128),
        fee_bps: None,
        fee_address: None,
        max_fee: None,
    };
    let info = mock_info("creator", &coins(2, "token"));
    contract::instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Agreement 1: Alice -> Bob
    let info = mock_info("alice", &coins(2, "token"));
    let msg = ExecuteMsg::CreateAgreement {
        receiver: String::from("bob"),
        amount: Uint256::from(1000000u128),
        start_at: None,
        end_at: None,
        interval: 86400,
    };
    let _res = contract::execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Agreement 2: Alice -> Charlie
    let info = mock_info("alice", &coins(2, "token"));
    let msg = ExecuteMsg::CreateAgreement {
        receiver: String::from("charlie"),
        amount: Uint256::from(100000000u128),
        start_at: None,
        end_at: None,
        interval: 86400,
    };
    let _res = contract::execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Agreement 3: Bob -> Charlie
    let info = mock_info("bob", &coins(2, "token"));
    let msg = ExecuteMsg::CreateAgreement {
        receiver: String::from("charlie"),
        amount: Uint256::from(9900000u128),
        start_at: None,
        end_at: None,
        interval: 86400,
    };
    let _res = contract::execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // query all agreements
    let msg = QueryMsg::AllAgreements {
        start_after: None,
        limit: None,
    };
    let res = contract::query(deps.as_ref(), mock_env(), msg).unwrap();
    let val: AgreementsResponse = from_binary(&res).unwrap();
    assert_eq!(
        val,
        AgreementsResponse {
            agreement_ids: vec![1, 2, 3],
            last_key: Some(3)
        }
    );

    // query owner agreements
    let msg = QueryMsg::AgreementsByOwner {
        owner: "alice".to_string(),
        start_after: None,
        limit: None,
    };
    let res = contract::query(deps.as_ref(), mock_env(), msg).unwrap();
    let val: AgreementsResponse = from_binary(&res).unwrap();
    assert_eq!(
        val,
        AgreementsResponse {
            agreement_ids: vec![1, 2],
            last_key: Some(2)
        }
    );

    // query receiver agreements
    let msg = QueryMsg::AgreementsByReceiver {
        receiver: "bob".to_string(),
        start_after: None,
        limit: None,
    };
    let res = contract::query(deps.as_ref(), mock_env(), msg).unwrap();
    let val: AgreementsResponse = from_binary(&res).unwrap();
    assert_eq!(
        val,
        AgreementsResponse {
            agreement_ids: vec![1],
            last_key: Some(1)
        }
    );

    // query limit & start_after agreements
    let msg = QueryMsg::AllAgreements {
        start_after: Some(1u64),
        limit: Some(2u32),
    };
    let res = contract::query(deps.as_ref(), mock_env(), msg).unwrap();
    let val: AgreementsResponse = from_binary(&res).unwrap();
    assert_eq!(
        val,
        AgreementsResponse {
            agreement_ids: vec![2, 3],
            last_key: Some(3)
        }
    );
}

#[test]
fn change_config() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        job_registry_contract: Some("job_registry".to_string()),
        minimum_interval: HOUR_SECONDS,
        minimum_amount_per_interval: Uint256::from(1_000_000u128),
        fee_bps: None,
        fee_address: None,
        max_fee: None,
    };
    let info = mock_info("creator", &coins(1000, "earth"));
    let res = contract::instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    let msg = QueryMsg::Config {};
    let res = contract::query(deps.as_ref(), mock_env(), msg).unwrap();
    let config: Config = from_binary(&res).unwrap();
    let expected_config: Config = Config {
        owner: Addr::unchecked("creator"),
        job_registry_contract: Some(Addr::unchecked("job_registry")),
        minimum_interval: HOUR_SECONDS,
        minimum_amount_per_interval: Uint256::from(1_000_000u128),
        fee_bps: 0u64,
        fee_address: Addr::unchecked("creator"),
        max_fee: Uint256::zero(),
    };
    assert_eq!(config, expected_config);

    let msg = ExecuteMsg::UpdateConfig {
        new_owner: Some(String::from("the_new_owner")),
        job_registry_contract: Some(String::from("the_new_job_registry")),
        minimum_interval: Some(DAY_SECONDS),
        minimum_amount_per_interval: None,
        fee_bps: Some(100u64),
        fee_address: Some(String::from("fee-collector")),
        max_fee: Some(Uint256::from(100_000u128)),
    };

    // unauthorised user attempts to change config. Should fail
    let info = mock_info("mallory", &coins(2, "token"));

    let res = contract::execute(deps.as_mut(), mock_env(), info, msg.clone());
    match res {
        Err(ContractError::Unauthorized {}) => {}
        _ => panic!("Contract should return an unauthorised error amount"),
    }

    // state should not change
    let query_msg = QueryMsg::Config {};
    let res = contract::query(deps.as_ref(), mock_env(), query_msg).unwrap();
    let config: Config = from_binary(&res).unwrap();
    let expected_config: Config = Config {
        owner: Addr::unchecked("creator"),
        job_registry_contract: Some(Addr::unchecked("job_registry")),
        minimum_interval: HOUR_SECONDS,
        minimum_amount_per_interval: Uint256::from(1_000_000u128),
        fee_bps: 0u64,
        fee_address: Addr::unchecked("creator"),
        max_fee: Uint256::zero(),
    };
    assert_eq!(config, expected_config);

    // only owner can change config
    let info = mock_info("creator", &coins(2, "token"));
    let _res = contract::execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = QueryMsg::Config {};
    let res = contract::query(deps.as_ref(), mock_env(), msg).unwrap();
    let config: Config = from_binary(&res).unwrap();
    let expected_config: Config = Config {
        owner: Addr::unchecked("the_new_owner"),
        job_registry_contract: Some(Addr::unchecked("the_new_job_registry")),
        minimum_interval: DAY_SECONDS,
        minimum_amount_per_interval: Uint256::from(1_000_000u128),
        fee_bps: 100u64,
        fee_address: Addr::unchecked("fee-collector"),
        max_fee: Uint256::from(100_000u128),
    };

    assert_eq!(config, expected_config);
}

#[test]
fn test_charge_lapsed_expiry() {
    let mut deps = mock_dependencies(&coins(2, "token"));
    let mut env = mock_env();

    let start_time = 1609459200u64;
    env.block.time = Timestamp::from_seconds(start_time);

    let msg = InstantiateMsg {
        job_registry_contract: Some("job_registry".to_string()),
        minimum_interval: DAY_SECONDS,
        minimum_amount_per_interval: Uint256::from(1_000_000u128),
        fee_bps: None,
        fee_address: None,
        max_fee: None,
    };
    let info = mock_info("creator", &coins(2, "token"));
    contract::instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Creates Alive -> Bob that starts in an hour and expires in a week
    let info = mock_info("alice", &coins(2, "token"));
    let msg = ExecuteMsg::CreateAgreement {
        receiver: String::from("bob"),
        amount: Uint256::from(1000000u128),
        start_at: Some(start_time + HOUR_SECONDS),
        end_at: Some(start_time + HOUR_SECONDS + DAY_SECONDS * 7),
        interval: DAY_SECONDS,
    };
    let _res = contract::execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Creates Bob -> Charlie that starts in 3 1hr days and expires in 4 days
    let info = mock_info("bob", &coins(2, "token"));
    let msg = ExecuteMsg::CreateAgreement {
        receiver: String::from("charlie"),
        amount: Uint256::from(1000000u128),
        start_at: Some(start_time + DAY_SECONDS * 3 + HOUR_SECONDS),
        end_at: Some(start_time + DAY_SECONDS * 4),
        interval: DAY_SECONDS,
    };
    let _res = contract::execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let val: AgreementResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Agreement { agreement_id: 1 },
        )
        .unwrap(),
    )
    .unwrap();

    // No charge when not started
    assert_eq!(
        val,
        AgreementResponse {
            created_at: start_time,
            interval_due_at: start_time + HOUR_SECONDS,
            start_at: start_time + HOUR_SECONDS,
            amount: Uint256::from(1000000u128),
            interval: DAY_SECONDS,
            from: Addr::unchecked("alice"),
            to: Addr::unchecked("bob"),
            end_at: Some(start_time + HOUR_SECONDS + DAY_SECONDS * 7,),
            status: AgreementStatus::NotStarted,
            pending_charge: Uint256::zero(),
            last_charged: start_time
        }
    );

    // Has pending charge after an hour
    env.block.time = Timestamp::from_seconds(start_time + HOUR_SECONDS);

    let val: AgreementResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Agreement { agreement_id: 1 },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        val,
        AgreementResponse {
            created_at: start_time,
            interval_due_at: start_time + HOUR_SECONDS,
            start_at: start_time + HOUR_SECONDS,
            amount: Uint256::from(1000000u128),
            interval: DAY_SECONDS,
            from: Addr::unchecked("alice"),
            to: Addr::unchecked("bob"),
            end_at: Some(start_time + HOUR_SECONDS + 7 * DAY_SECONDS,),
            status: AgreementStatus::Active,
            pending_charge: Uint256::from(1000000u128),
            last_charged: start_time
        }
    );

    // Execute transfers
    let info = mock_info("bot", &[]);
    let msg = ExecuteMsg::Transfer { agreement_id: 1u64 };
    let res = contract::execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("method", "execute_transfer"),
            attr("agreement_id", "1"),
            attr("amount", "1000000"),
        ]
    );

    let val: AgreementResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Agreement { agreement_id: 1 },
        )
        .unwrap(),
    )
    .unwrap();

    // Resets pending_charge upon successful transfers
    assert_eq!(
        val,
        AgreementResponse {
            created_at: start_time,
            interval_due_at: start_time + HOUR_SECONDS + DAY_SECONDS,
            start_at: start_time + HOUR_SECONDS,
            amount: Uint256::from(1000000u128),
            interval: DAY_SECONDS,
            from: Addr::unchecked("alice"),
            to: Addr::unchecked("bob"),
            end_at: Some(start_time + HOUR_SECONDS + DAY_SECONDS * 7,),
            status: AgreementStatus::Active,
            pending_charge: Uint256::from(0u128),
            last_charged: start_time + HOUR_SECONDS
        }
    );

    // Elaspsed an entire interval duration
    env.block.time = Timestamp::from_seconds(start_time + HOUR_SECONDS + 2 * DAY_SECONDS + 1);

    let val: AgreementResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Agreement { agreement_id: 1 },
        )
        .unwrap(),
    )
    .unwrap();

    // Agreement has lapsed and failed
    assert_eq!(
        val,
        AgreementResponse {
            created_at: start_time,
            interval_due_at: start_time + HOUR_SECONDS + DAY_SECONDS,
            start_at: start_time + HOUR_SECONDS,
            amount: Uint256::from(1000000u128),
            interval: DAY_SECONDS,
            from: Addr::unchecked("alice"),
            to: Addr::unchecked("bob"),
            end_at: Some(start_time + HOUR_SECONDS + DAY_SECONDS * 7,),
            status: AgreementStatus::Lapsed,
            pending_charge: Uint256::from(0u128),
            last_charged: start_time + HOUR_SECONDS
        }
    );

    // Cannot charge lapsed account
    let msg = ExecuteMsg::Transfer { agreement_id: 1u64 };
    let info = mock_info("alice", &coins(2, "token"));
    let res = contract::execute(deps.as_mut(), env.clone(), info, msg.clone());
    match res {
        Err(ContractError::ZeroTransferableAmount {}) => {}
        _ => panic!("Contract should return a zero transferable amount"),
    }

    // Expired by time
    env.block.time = Timestamp::from_seconds(start_time + DAY_SECONDS * 4);
    let val: AgreementResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Agreement { agreement_id: 2 },
        )
        .unwrap(),
    )
    .unwrap();

    // Agreement has expired
    assert_eq!(
        val,
        AgreementResponse {
            created_at: start_time,
            interval_due_at: start_time + 3 * DAY_SECONDS + HOUR_SECONDS,
            start_at: start_time + 3 * DAY_SECONDS + HOUR_SECONDS,
            amount: Uint256::from(1000000u128),
            interval: DAY_SECONDS,
            from: Addr::unchecked("bob"),
            to: Addr::unchecked("charlie"),
            end_at: Some(start_time + DAY_SECONDS * 4),
            status: AgreementStatus::Expired,
            pending_charge: Uint256::from(0u128),
            last_charged: start_time
        }
    );
}

#[test]
fn test_query_overdued() {
    let mut deps = mock_dependencies(&coins(2, "token"));
    let mut env = mock_env();

    let start_time = 1609459200u64;
    env.block.time = Timestamp::from_seconds(start_time);

    let msg = InstantiateMsg {
        job_registry_contract: Some("job_registry".to_string()),
        minimum_interval: DAY_SECONDS,
        minimum_amount_per_interval: Uint256::from(1_000_000u128),
        fee_bps: None,
        fee_address: None,
        max_fee: None,
    };
    let info = mock_info("creator", &coins(2, "token"));
    contract::instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // #1. Creates Alice -> Bob that starts almost immediately
    let info = mock_info("alice", &coins(2, "token"));
    let msg = ExecuteMsg::CreateAgreement {
        receiver: String::from("bob"),
        amount: Uint256::from(1000000u128),
        start_at: Some(start_time + 1), // No initial amount charged
        end_at: None,
        interval: DAY_SECONDS,
    };
    contract::execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // #2. Creates Alice -> Bob that starts in 2 hours
    let info = mock_info("alice", &coins(2, "token"));
    let msg = ExecuteMsg::CreateAgreement {
        receiver: String::from("bob"),
        amount: Uint256::from(2000000u128),
        start_at: Some(start_time + 2 * HOUR_SECONDS),
        end_at: None,
        interval: DAY_SECONDS,
    };
    contract::execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // #3. Creates Alice -> Bob that starts in 1 hour
    let info = mock_info("alice", &coins(2, "token"));
    let msg = ExecuteMsg::CreateAgreement {
        receiver: String::from("bob"),
        amount: Uint256::from(3000000u128),
        start_at: Some(start_time + HOUR_SECONDS),
        end_at: None,
        interval: DAY_SECONDS,
    };
    contract::execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    env.block.time = Timestamp::from_seconds(start_time + 3 * HOUR_SECONDS);

    // #4. After 3 hours, creates Alice -> Bob that starts immediately
    let info = mock_info("alice", &coins(2, "token"));
    let msg = ExecuteMsg::CreateAgreement {
        receiver: String::from("bob"),
        amount: Uint256::from(3000000u128),
        start_at: None,
        end_at: None,
        interval: DAY_SECONDS,
    };
    contract::execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // query overdued agreements
    let msg = QueryMsg::OverduedAgreements {
        start_after: None,
        limit: None,
    };
    let res = contract::query(deps.as_ref(), env.clone(), msg).unwrap();
    let val: AgreementsResponse = from_binary(&res).unwrap();
    assert_eq!(val.agreement_ids, [1, 3, 2]);

    // query overdued agreements paginate
    let msg = QueryMsg::OverduedAgreements {
        start_after: Some(1609459201),
        limit: Some(2),
    };
    let res = contract::query(deps.as_ref(), env.clone(), msg).unwrap();
    let val: AgreementsResponse = from_binary(&res).unwrap();
    assert_eq!(val.agreement_ids, [3, 2]);

    // Charge agreement 1
    let info = mock_info("bot", &coins(2, "token"));
    contract::execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::Transfer { agreement_id: 1 },
    )
    .unwrap();

    // query overdued agreements
    let msg = QueryMsg::OverduedAgreements {
        start_after: None,
        limit: None,
    };
    let res = contract::query(deps.as_ref(), env.clone(), msg).unwrap();
    let val: AgreementsResponse = from_binary(&res).unwrap();
    assert_eq!(
        val,
        AgreementsResponse {
            agreement_ids: vec![3, 2],
            last_key: Some(1609466400)
        }
    );

    // Agreements lapse when overdue time exceeds a whole interval
    // More than 24 hours lapsed, #2,#3 agreements has lapsed
    env.block.time = Timestamp::from_seconds(start_time + 3 * HOUR_SECONDS + DAY_SECONDS);

    // query overdued agreements that's not lapsed
    let msg = QueryMsg::OverduedAgreements {
        start_after: None,
        limit: None,
    };
    let res = contract::query(deps.as_ref(), env.clone(), msg).unwrap();
    let val: AgreementsResponse = from_binary(&res).unwrap();
    assert_eq!(
        val,
        AgreementsResponse {
            agreement_ids: vec![1, 4],
            last_key: Some(start_time + 3 * HOUR_SECONDS + DAY_SECONDS)
        }
    );

    println!("Current time: {:?}", env.block.time.seconds());
    for i in 1..=4 {
        let res = contract::query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Agreement { agreement_id: i },
        )
        .unwrap();
        let val: AgreementResponse = from_binary(&res).unwrap();
        println!(
            "Agreement {:?}, status {:?}, due {:?}, delta {:?}s ago",
            i,
            val.status,
            val.interval_due_at,
            env.block.time.seconds().saturating_sub(val.interval_due_at)
        );
    }
}
