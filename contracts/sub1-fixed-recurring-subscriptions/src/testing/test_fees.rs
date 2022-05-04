use crate::contract;
use crate::mock_querier::mock_dependencies;
use crate::msg::ExecuteMsg;

use cosmwasm_bignumber::Uint256;
use cosmwasm_std::testing::{mock_env, mock_info};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{attr, to_binary, Coin, ReplyOn, SubMsg, Timestamp, Uint128, WasmMsg};
use suberra_core::msg::{ProductInstantiateMsg, SubWalletExecuteMsg};

const THIRTY_DAYS_IN_SECONDS: u64 = 60 * 60 * 720;

#[test]
fn subscribe_with_fees() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(100u128),
    }]);

    // test with 1% fee
    deps.querier
        .with_fee(100, Uint256::zero(), Uint256::from(10_000_000u64), 24u64);

    let msg = ProductInstantiateMsg {
        receiver_address: "merchant".to_string(),
        unit_amount: Uint256::from(1000u128),
        initial_amount: Uint256::from(1000u128),
        unit_interval_hour: 720u64,
        additional_grace_period_hour: None,
        admins: Vec::new(),
        mutable: false,
        uri: "{\"image_url\": \"www.google.com\" }".to_string(),
        factory_address: "factory".to_string(),
        owner: "creator".to_string(),
    };

    let mut env = mock_env();
    let start_timestamp = 1609459200;
    env.block.time = Timestamp::from_seconds(start_timestamp); // set to 1 January 2021 00:00:00 GMT

    let info = mock_info(
        "creator",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );

    let _res = contract::instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let subscriber1 = mock_info(
        "subscriber",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(10000u128),
        }],
    );

    // user tries to subscribe

    let msg = ExecuteMsg::Subscribe {};
    let res =
        contract::execute(deps.as_mut(), env.clone(), subscriber1.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("initial_amount", "1000"),
            attr("method", "execute_subscribe"),
            attr("result", "subscribe_success"),
            attr("subscriber", "subscriber"),
            attr("module_contract_address", "cosmos2contract"),
        ]
    );

    assert_eq!(res.messages.len(), 2);

    let msg_protocol_transfer = res.messages.get(0).expect("no message");
    let msg_recipient_transfer = res.messages.get(1).expect("no message");

    assert_eq!(
        msg_protocol_transfer,
        &SubMsg {
            msg: WasmMsg::Execute {
                contract_addr: String::from("subscriber"),
                msg: to_binary(&SubWalletExecuteMsg::TransferAToken {
                    recipient: String::from("fee_address"),
                    amount: Uint128::from(10u128),
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
            id: 0,
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );
    assert_eq!(
        msg_recipient_transfer,
        &SubMsg {
            msg: WasmMsg::Execute {
                contract_addr: String::from("subscriber"),
                msg: to_binary(&SubWalletExecuteMsg::TransferAToken {
                    recipient: String::from("merchant"),
                    amount: Uint128::from(990u128),
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
            id: 0,
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );
}

#[test]
fn test_charge() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(100u128),
    }]);

    // test with 0.5% fee
    deps.querier
        .with_fee(50, Uint256::zero(), Uint256::from(10_000_000u64), 24u64);

    let msg = ProductInstantiateMsg {
        receiver_address: "merchant".to_string(),
        unit_amount: Uint256::from(1000u128),
        initial_amount: Uint256::from(1000u128),
        unit_interval_hour: 720u64,
        additional_grace_period_hour: None,
        admins: Vec::new(),
        mutable: false,
        uri: "{\"image_url\": \"www.google.com\" }".to_string(),
        factory_address: "factory".to_string(),
        owner: "creator".to_string(),
    };

    let mut env = mock_env();
    let start_timestamp = 1609459200;
    env.block.time = Timestamp::from_seconds(start_timestamp); // set to 1 January 2021 00:00:00 GMT

    let info = mock_info(
        "creator",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );

    let _res = contract::instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let subscriber1 = mock_info(
        "subscriber",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(10000u128),
        }],
    );

    // user tries to subscribe

    let msg = ExecuteMsg::Subscribe {};
    let _res =
        contract::execute(deps.as_mut(), env.clone(), subscriber1.clone(), msg.clone()).unwrap();

    let info = mock_info("charger", &[]);
    // fast-forwards timestamp by one billing cycle (720 hours in seconds);
    let new_timestamp = start_timestamp + THIRTY_DAYS_IN_SECONDS;

    env.block.time = Timestamp::from_seconds(new_timestamp);

    let msg = ExecuteMsg::Charge {
        payer_address: "subscriber".to_string(),
    };

    let res = contract::execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("method", "execute_charge"),
            attr("module_contract_address", "cosmos2contract"),
            attr("subscriber", "subscriber"),
            attr("amount", "1000"),
            attr("periods", "1")
        ]
    );

    assert_eq!(res.messages.len(), 2);

    let msg_protocol_transfer = res.messages.get(0).expect("no message");
    let msg_recipient_transfer = res.messages.get(1).expect("no message");

    assert_eq!(
        msg_protocol_transfer,
        &SubMsg {
            msg: WasmMsg::Execute {
                contract_addr: String::from("subscriber"),
                msg: to_binary(&SubWalletExecuteMsg::TransferAToken {
                    recipient: String::from("fee_address"),
                    amount: Uint128::from(5u128),
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
            id: 0,
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );
    assert_eq!(
        msg_recipient_transfer,
        &SubMsg {
            msg: WasmMsg::Execute {
                contract_addr: String::from("subscriber"),
                msg: to_binary(&SubWalletExecuteMsg::TransferAToken {
                    recipient: String::from("merchant"),
                    amount: Uint128::from(995u128),
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
            id: 0,
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );
}

#[test]
fn test_charge_with_min_fees() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(100u128),
    }]);

    // test with 0.5% fee
    deps.querier.with_fee(
        1,
        Uint256::from(100u128),
        Uint256::from(10_000_000u64),
        24u64,
    );

    let msg = ProductInstantiateMsg {
        receiver_address: "merchant".to_string(),
        unit_amount: Uint256::from(1000u128),
        initial_amount: Uint256::from(1000u128),
        unit_interval_hour: 720u64,
        additional_grace_period_hour: None,
        owner: "creator".to_string(),
        admins: Vec::new(),
        mutable: false,
        uri: "{\"image_url\": \"www.google.com\" }".to_string(),
        factory_address: "factory".to_string(),
    };

    let mut env = mock_env();
    let start_timestamp = 1609459200;
    env.block.time = Timestamp::from_seconds(start_timestamp); // set to 1 January 2021 00:00:00 GMT

    let info = mock_info(
        "creator",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );

    let _res = contract::instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let subscriber1 = mock_info(
        "subscriber",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(10000u128),
        }],
    );

    // user tries to subscribe

    let msg = ExecuteMsg::Subscribe {};
    let _res =
        contract::execute(deps.as_mut(), env.clone(), subscriber1.clone(), msg.clone()).unwrap();

    let info = mock_info("charger", &[]);
    // fast-forwards timestamp by one billing cycle (720 hours in seconds);
    let new_timestamp = start_timestamp + THIRTY_DAYS_IN_SECONDS;

    env.block.time = Timestamp::from_seconds(new_timestamp);

    let msg = ExecuteMsg::Charge {
        payer_address: "subscriber".to_string(),
    };

    let res = contract::execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("method", "execute_charge"),
            attr("module_contract_address", "cosmos2contract"),
            attr("subscriber", "subscriber"),
            attr("amount", "1000"),
            attr("periods", "1")
        ]
    );

    assert_eq!(res.messages.len(), 2);

    let msg_protocol_transfer = res.messages.get(0).expect("no message");
    let msg_recipient_transfer = res.messages.get(1).expect("no message");

    assert_eq!(
        msg_protocol_transfer,
        &SubMsg {
            msg: WasmMsg::Execute {
                contract_addr: String::from("subscriber"),
                msg: to_binary(&SubWalletExecuteMsg::TransferAToken {
                    recipient: String::from("fee_address"),
                    amount: Uint128::from(100u128),
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
            id: 0,
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );
    assert_eq!(
        msg_recipient_transfer,
        &SubMsg {
            msg: WasmMsg::Execute {
                contract_addr: String::from("subscriber"),
                msg: to_binary(&SubWalletExecuteMsg::TransferAToken {
                    recipient: String::from("merchant"),
                    amount: Uint128::from(900u128),
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
            id: 0,
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );
}
