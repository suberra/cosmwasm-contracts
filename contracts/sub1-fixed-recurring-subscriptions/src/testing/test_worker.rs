use crate::contract;

use crate::msg::ExecuteMsg;
use cosmwasm_bignumber::Uint256;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{from_binary, to_binary, Coin, Timestamp, Uint128};
use suberra_core::msg::ProductInstantiateMsg;

use crate::mock_querier::mock_dependencies;
use crate::msg::{QueryMsg, WorkPayload};
use cosmwasm_std::testing::{mock_env, mock_info};

#[test]
fn query_can_work() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(100u128),
    }]);
    let msg = ProductInstantiateMsg {
        receiver_address: "receiver".to_string(),
        unit_amount: Uint256::from(123u128),
        initial_amount: Uint256::from(123u128),
        unit_interval_hour: 720u64,
        additional_grace_period_hour: None,
        max_amount_chargeable: Some(Uint256::from(123u128)),
        admins: Vec::new(),
        mutable: false,
        uri: "{\"image_url\": \"www.google.com\" }".to_string(),
        factory_address: "factory".to_string(),
        owner: "creator".to_string(),
    };

    let info = mock_info(
        "creator",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );

    let _res = contract::instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // set env
    let mut env = mock_env();
    let start_timestamp = 1609459200;
    env.block.time = Timestamp::from_seconds(start_timestamp); // set to 1 January 2021 00:00:00 GMT

    let info_subscriber = mock_info(
        "subscriber",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );

    // user tries to subscribe

    let msg = ExecuteMsg::Subscribe {};
    let _ = contract::execute(deps.as_mut(), env.clone(), info_subscriber.clone(), msg);

    // fast forward to 300 hours later
    let mut new_timestamp = start_timestamp + 300 * 60 * 60;
    env.block.time = Timestamp::from_seconds(new_timestamp);

    let work_payload = WorkPayload {
        payer_address: "subscriber".to_string(),
    };

    let result: bool = from_binary(
        &contract::query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::CanWork {
                payload: to_binary(&work_payload).unwrap(),
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(result, false);

    new_timestamp = start_timestamp + (720 * 60 * 60) + 1; // January 31, 2021 12:00:01 AM
    env.block.time = Timestamp::from_seconds(new_timestamp);
    let result: bool = from_binary(
        &contract::query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::CanWork {
                payload: to_binary(&work_payload).unwrap(),
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(result, true);
}
