use crate::contract;

use crate::error::ContractError;
use crate::mock_querier::mock_dependencies;
use crate::msg::{
    ConfigResponse, ExecuteMsg, QueryMsg, SubscriptionInfoResponse, SubscriptionsResponse,
};
use crate::state::{Config, SubscriptionInfo};
use admin_core::msg::AdminConfigResponse;
use cosmwasm_bignumber::Uint256;
use cosmwasm_std::testing::{mock_env, mock_info};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{attr, from_binary, Addr, Api, Coin, Timestamp, Uint128};
use suberra_core::msg::ProductInstantiateMsg;
use suberra_core::subscriptions::Discount;

const DEFAULT_GRACE_PERIOD: u64 = 86400; // 24 hours in seconds
const THIRTY_DAYS_IN_SECONDS: u64 = 60 * 60 * 720;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(100u128),
    }]);

    let msg = ProductInstantiateMsg {
        receiver_address: "receiver".to_string(),
        additional_grace_period_hour: None,
        unit_amount: Uint256::from(123u128),
        initial_amount: Uint256::from(123u128),
        unit_interval_hour: 2592000u64,
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

    // we can just call .unwrap() to assert this was a success
    let res = contract::instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(0, res.messages.len());

    let res = contract::query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let value: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(false, value.paused);
}

#[test]
fn pause() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(100u128),
    }]);
    let msg = ProductInstantiateMsg {
        receiver_address: "receiver".to_string(),
        initial_amount: Uint256::from(123u128),
        unit_amount: Uint256::from(123u128),
        unit_interval_hour: 2592000u64,
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

    // starting state should be paused
    let res = contract::query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let value: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(false, value.paused);

    // contract cannot be pause or unpaused by an unauthorised personnel
    let unauth_info = mock_info(
        "anyone",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    let msg = ExecuteMsg::Pause {};
    let res = contract::execute(deps.as_mut(), mock_env(), unauth_info.clone(), msg);
    match res {
        Err(ContractError::Unauthorized {}) => {}
        _ => panic!("Must return unauthorized error"),
    }

    // only the original creator can pause
    let msg = ExecuteMsg::Pause {};
    let _res = contract::execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // state should now be set to pause
    let res = contract::query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let value: ConfigResponse = from_binary(&res).unwrap();

    let expected_config = ConfigResponse {
        owner_address: "creator".to_string(),
        receiver_address: "receiver".to_string(),
        initial_amount: Uint256::from(123u128),
        additional_grace_period: 0,
        unit_amount: Uint256::from(123u128),
        unit_interval_seconds: 2592000 * 60 * 60,
        uri: "{\"image_url\": \"www.google.com\" }".to_string(),
        paused: true,
    };

    assert_eq!(expected_config, value);

    let res = contract::execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::Charge {
            payer_address: "subscriber1".to_string(),
        },
    );
    match res {
        Err(ContractError::Paused {}) => {}
        _ => panic!("Must return unauthorized error"),
    };

    let res = contract::execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::Subscribe {},
    );
    match res {
        Err(ContractError::Paused {}) => {}
        _ => panic!("Must return unauthorized error"),
    }
}

#[test]
fn unpause() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(100u128),
    }]);
    let msg = ProductInstantiateMsg {
        owner: "creator".to_string(),
        receiver_address: "receiver".to_string(),
        initial_amount: Uint256::from(123u128),
        unit_amount: Uint256::from(123u128),
        unit_interval_hour: 2592000u64,
        additional_grace_period_hour: None,
        max_amount_chargeable: Some(Uint256::from(123u128)),
        admins: Vec::new(),
        mutable: false,
        uri: "{\"image_url\": \"www.google.com\" }".to_string(),
        factory_address: "factory".to_string(),
    };

    let info = mock_info(
        "creator",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );

    let _res = contract::instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    // only the original creator can pause
    let msg = ExecuteMsg::Pause {};

    let _res = contract::execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
    // state should now be set to pause
    let res = contract::query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let value: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(value.paused, true);

    // contract should not be paused again

    let res = contract::execute(deps.as_mut(), mock_env(), info.clone(), msg.clone());
    match res {
        Err(ContractError::Paused {}) => {}
        _ => panic!("Must return unauthorized error"),
    }

    // unpause
    let msg = ExecuteMsg::Unpause {};
    let _res = contract::execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    // state should now be set to pause
    let res = contract::query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let value: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(value.paused, false); // contract should now be unpaused
}

#[test]
fn simple_subscribe() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(100u128),
    }]);
    let msg = ProductInstantiateMsg {
        owner: "creator".to_string(),
        receiver_address: "receiver".to_string(),
        unit_amount: Uint256::from(123u128),
        initial_amount: Uint256::from(123u128),
        unit_interval_hour: 720u64,
        additional_grace_period_hour: None,
        max_amount_chargeable: Some(Uint256::from(400u128)),
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
            amount: Uint128::from(100u128),
        }],
    );

    // user tries to subscribe

    let msg = ExecuteMsg::Subscribe {};
    let res =
        contract::execute(deps.as_mut(), env.clone(), subscriber1.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("initial_amount", "123"),
            attr("method", "execute_subscribe"),
            attr("result", "subscribe_success"),
            attr("subscriber", "subscriber"),
            attr("module_contract_address", "cosmos2contract"),
        ]
    );

    assert_eq!(res.messages.len(), 1);

    let subscriptions: SubscriptionsResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Subscriptions {
                limit: None,
                start_after: None,
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(subscriptions.subscriptions.len(), 1);

    // second subscriber
    let subscriber2 = mock_info(
        "subscriber2",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    let _res =
        contract::execute(deps.as_mut(), env.clone(), subscriber2.clone(), msg.clone()).unwrap();

    let subscriptions: SubscriptionsResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Subscriptions {
                limit: None,
                start_after: None,
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(subscriptions.subscriptions.len(), 2);

    // fast-forwards timestamp by one billing cycle and check that the subscription is still alive
    let info = mock_info("charger", &[]);
    env.block.time = Timestamp::from_seconds(start_timestamp + THIRTY_DAYS_IN_SECONDS);

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
            attr("amount", "123"),
            attr("periods", "1")
        ]
    );

    // Test the query subscriber

    let res = contract::query(
        deps.as_ref(),
        env.clone(),
        QueryMsg::Subscription {
            subscriber: "subscriber".to_string(),
        },
    )
    .unwrap();

    let subscriber_info: SubscriptionInfoResponse = from_binary(&res).unwrap();
    let expected_subscription = SubscriptionInfoResponse {
        subscriber: "subscriber".to_string(),
        created_at: start_timestamp,
        last_charged: start_timestamp + THIRTY_DAYS_IN_SECONDS,
        interval_end_at: start_timestamp + 2 * THIRTY_DAYS_IN_SECONDS,
        is_cancelled: false,
        is_active: true,
        discount_per_interval: None,
        amount_chargeable: Some(Uint256::zero()),
    };

    // subscription should be cancelled
    assert_eq!(subscriber_info, expected_subscription);

    // fast forwards two billing cycles
    let next_timestamp = subscriber_info.interval_end_at + THIRTY_DAYS_IN_SECONDS + 1u64; // add 1 second so it falls within the second interval
    env.block.time = Timestamp::from_seconds(next_timestamp);

    let msg = ExecuteMsg::Charge {
        payer_address: "subscriber".to_string(),
    };

    let res = contract::execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    match res {
        Err(ContractError::CannotCharge {}) => {}
        _ => panic!("Must return cannot charge error"),
    }
}

#[test]
fn query_subscriber() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(150u128),
    }]);
    let msg = ProductInstantiateMsg {
        owner: "creator".to_string(),
        receiver_address: "receiver".to_string(),
        unit_amount: Uint256::from(123u128),
        initial_amount: Uint256::from(123u128),
        unit_interval_hour: 720u64,
        additional_grace_period_hour: Some(720u64),
        max_amount_chargeable: Some(Uint256::from(123u128)),
        admins: Vec::new(),
        mutable: false,
        uri: "{\"image_url\": \"www.google.com\" }".to_string(),
        factory_address: "factory".to_string(),
    };

    let info = mock_info(
        "creator",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );

    let _res = contract::instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let subscriber1 = mock_info(
        "subscriber",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );

    let mut env = mock_env();
    let start_timestamp_seconds = 1609459200; // set to 1 January 2021 00:00:00 GMT
    env.block.time = Timestamp::from_seconds(start_timestamp_seconds);

    // user tries to subscribe
    let msg = ExecuteMsg::Subscribe {};
    let res =
        contract::execute(deps.as_mut(), env.clone(), subscriber1.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("initial_amount", "123"),
            attr("method", "execute_subscribe"),
            attr("result", "subscribe_success"),
            attr("subscriber", "subscriber"),
            attr("module_contract_address", "cosmos2contract"),
        ]
    );

    assert_eq!(res.messages.len(), 1);

    let res = contract::query(
        deps.as_ref(),
        env.clone(),
        QueryMsg::Subscription {
            subscriber: "subscriber".to_string(),
        },
    )
    .unwrap();

    let subscriber_info: SubscriptionInfoResponse = from_binary(&res).unwrap();

    assert_eq!(subscriber_info.is_active, true);

    // fast forward by exactly 30 days (2592000 seconds)
    env.block.time = Timestamp::from_seconds(start_timestamp_seconds + 2592000);

    let res = contract::query(
        deps.as_ref(),
        env.clone(),
        QueryMsg::Subscription {
            subscriber: "subscriber".to_string(),
        },
    )
    .unwrap();

    let subscriber_info: SubscriptionInfoResponse = from_binary(&res).unwrap();

    assert_eq!(subscriber_info.is_active, true);

    // test with threshold, should return true subscription
    env.block.time = Timestamp::from_seconds(start_timestamp_seconds + 5184000);

    let res = contract::query(
        deps.as_ref(),
        env.clone(),
        QueryMsg::Subscription {
            subscriber: "subscriber".to_string(),
        },
    )
    .unwrap();

    let subscriber_info: SubscriptionInfoResponse = from_binary(&res).unwrap();

    assert_eq!(subscriber_info.is_active, true);

    // timestamp exceeds the threshold. Should return false

    env.block.time =
        Timestamp::from_seconds(start_timestamp_seconds + 5184001 + DEFAULT_GRACE_PERIOD);

    let res = contract::query(
        deps.as_ref(),
        env.clone(),
        QueryMsg::Subscription {
            subscriber: "subscriber".to_string(),
        },
    )
    .unwrap();

    let subscriber_info: SubscriptionInfoResponse = from_binary(&res).unwrap();

    assert_eq!(subscriber_info.is_active, false); // should return false since the threshold/grace period is over
}

#[test]
fn subscribe_multiple_times() {
    let mut deps = mock_dependencies(&[]);

    let msg = ProductInstantiateMsg {
        owner: "creator".to_string(),
        receiver_address: "receiver".to_string(),
        additional_grace_period_hour: None,
        unit_amount: Uint256::from(123u128),
        initial_amount: Uint256::from(123u128),
        unit_interval_hour: 720u64,
        max_amount_chargeable: Some(Uint256::from(123u128)),
        admins: Vec::new(),
        mutable: false,
        uri: "{\"image_url\": \"www.google.com\" }".to_string(),
        factory_address: "factory".to_string(),
    };

    let info = mock_info("creator", &[]);

    let _res = contract::instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let info_subscriber = mock_info(
        "subscriber",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );

    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(1609459200); // set to 1 January 2021 00:00:00 GMT

    // user tries to subscribe for the first time

    let msg = ExecuteMsg::Subscribe {};
    let res = contract::execute(
        deps.as_mut(),
        env.clone(),
        info_subscriber.clone(),
        msg.clone(),
    )
    .unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("initial_amount", "123"),
            attr("method", "execute_subscribe"),
            attr("result", "subscribe_success"),
            attr("subscriber", "subscriber"),
            attr("module_contract_address", "cosmos2contract"),
        ]
    );

    // checks query susbcription for the correct values
    let subscription: SubscriptionInfoResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Subscription {
                subscriber: "subscriber".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();

    // subscription object should be created
    assert_eq!(subscription.subscriber, "subscriber");
    assert_eq!(subscription.is_active, true);

    let prev_timestamp = subscription.created_at;

    // user tries to subscribe again despite having an active subscription

    env.block.time = Timestamp::from_seconds(1610668800); // 15 January 2021 00:00:00 GMT

    let res = contract::execute(
        deps.as_mut(),
        env.clone(),
        info_subscriber.clone(),
        msg.clone(),
    );
    match res {
        Err(ContractError::ExistingSubscriptionFound {}) => {
            // check if the subscription is changed. Timestamp of the current subscription should still be the old timestamp
            let subscription: SubscriptionInfoResponse = from_binary(
                &contract::query(
                    deps.as_ref(),
                    env.clone(),
                    QueryMsg::Subscription {
                        subscriber: "subscriber".to_string(),
                    },
                )
                .unwrap(),
            )
            .unwrap();

            // timestamp should still be the old timestamp
            assert_eq!(subscription.created_at, prev_timestamp);
        }
        _ => panic!("Contract should return an error"),
    }

    // User tries to subscribe again after the previous subscription has finished.
    // A new subscription object should be created.

    let new_timestamp = 1614470400; // 28 February 2021 00:00:00 GMT;
    env.block.time = Timestamp::from_seconds(new_timestamp);
    let res = contract::execute(
        deps.as_mut(),
        env.clone(),
        info_subscriber.clone(),
        msg.clone(),
    )
    .unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("initial_amount", "123"),
            attr("method", "execute_subscribe"),
            attr("result", "subscribe_success"),
            attr("subscriber", "subscriber"),
            attr("module_contract_address", "cosmos2contract"),
        ]
    );

    let subscription: SubscriptionInfoResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Subscription {
                subscriber: "subscriber".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();

    // created_at timestamp should now be the new timestamp since its a new Subscription object
    assert_eq!(subscription.created_at, new_timestamp);
}

/// unpaid subscription should become invalid after it passes the threshold dates
/// query can_work should not return a positive value for amount chargeable since the subscription is no longer valid
#[test]
fn unpaid_subscriptions() {
    let mut deps = mock_dependencies(&[]);

    let msg = ProductInstantiateMsg {
        owner: "creator".to_string(),
        receiver_address: "receiver".to_string(),
        additional_grace_period_hour: None,
        unit_amount: Uint256::from(123u128),
        initial_amount: Uint256::from(123u128),
        unit_interval_hour: 720u64,
        max_amount_chargeable: Some(Uint256::from(123u128)),
        admins: Vec::new(),
        mutable: false,
        uri: "{\"image_url\": \"www.google.com\" }".to_string(),
        factory_address: "factory".to_string(),
    };

    let info = mock_info("creator", &[]);

    let _res = contract::instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let info_subscriber = mock_info(
        "subscriber",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );

    let mut env = mock_env();
    let start_timestamp = 1609459200;
    env.block.time = Timestamp::from_seconds(start_timestamp); // set to 1 January 2021 00:00:00 GMT

    // user tries to subscribe for the first time

    let msg = ExecuteMsg::Subscribe {};
    let res = contract::execute(
        deps.as_mut(),
        env.clone(),
        info_subscriber.clone(),
        msg.clone(),
    )
    .unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("initial_amount", "123"),
            attr("method", "execute_subscribe"),
            attr("result", "subscribe_success"),
            attr("subscriber", "subscriber"),
            attr("module_contract_address", "cosmos2contract"),
        ]
    );

    // user subscription should expire
    // checks query susbcription for the correct values
    let subscription: SubscriptionInfoResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Subscription {
                subscriber: "subscriber".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();

    // subscription object should be created
    assert_eq!(subscription.subscriber, "subscriber");
    assert_eq!(subscription.is_active, true);

    // fastforward 720 hours
    let mut new_timestamp = start_timestamp + THIRTY_DAYS_IN_SECONDS + 1 + DEFAULT_GRACE_PERIOD;
    env.block.time = Timestamp::from_seconds(new_timestamp);

    let subscription: SubscriptionInfoResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Subscription {
                subscriber: "subscriber".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(subscription.is_active, false); // should be false since subscription expired
    assert_eq!(subscription.amount_chargeable, Some(Uint256::zero()));

    // fastforward 1500 hours
    new_timestamp = start_timestamp + (1500 * 60 * 60) + 1;
    env.block.time = Timestamp::from_seconds(new_timestamp);

    let subscription: SubscriptionInfoResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Subscription {
                subscriber: "subscriber".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(subscription.is_active, false); // should be false since subscription expired
    assert_eq!(subscription.amount_chargeable, Some(Uint256::zero()));
}
#[test]
fn cancel_subscription() {
    let mut deps = mock_dependencies(&[]);
    let msg = ProductInstantiateMsg {
        owner: "creator".to_string(),
        receiver_address: "receiver".to_string(),
        unit_amount: Uint256::from(123u128),
        initial_amount: Uint256::from(123u128),
        unit_interval_hour: 720u64,
        additional_grace_period_hour: Some(259200u64),
        max_amount_chargeable: Some(Uint256::from(123u128)),
        admins: Vec::new(),
        mutable: false,
        uri: "{\"image_url\": \"www.google.com\" }".to_string(),
        factory_address: "factory".to_string(),
    };

    let info = mock_info("creator", &[]);

    let _res = contract::instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let info_subscriber = mock_info("subscriber", &[]);

    let mut env = mock_env();
    let start_timestamp = 1609459200;
    env.block.time = Timestamp::from_seconds(start_timestamp); // set to 1 January 2021 00:00:00 GMT

    let msg = ExecuteMsg::Subscribe {};
    let res = contract::execute(
        deps.as_mut(),
        env.clone(),
        info_subscriber.clone(),
        msg.clone(),
    )
    .unwrap();

    // subscribe should be success
    assert_eq!(
        res.attributes,
        vec![
            attr("initial_amount", "123"),
            attr("method", "execute_subscribe"),
            attr("result", "subscribe_success"),
            attr("subscriber", "subscriber"),
            attr("module_contract_address", "cosmos2contract"),
        ]
    );

    // subscriber attempts to cancel. This should succeed
    let msg = ExecuteMsg::Cancel {};
    let res = contract::execute(
        deps.as_mut(),
        env.clone(),
        info_subscriber.clone(),
        msg.clone(),
    )
    .unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("method", "execute_cancel"),
            attr("module_contract_address", "cosmos2contract")
        ]
    );

    let subscription: SubscriptionInfoResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Subscription {
                subscriber: "subscriber".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();

    // subscription should be cancelled
    assert_eq!(subscription.is_cancelled, true);
    assert_eq!(subscription.is_active, true); // subscription should still be active even though it is cancelled

    // charge should not be successful

    let new_timestamp = start_timestamp + THIRTY_DAYS_IN_SECONDS + 1; // January 31, 2021 12:00:01 AM
    env.block.time = Timestamp::from_seconds(new_timestamp);

    let subscription: SubscriptionInfoResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Subscription {
                subscriber: "subscriber".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();

    // subscription should be cancelled
    assert_eq!(subscription.is_cancelled, true);
    assert_eq!(subscription.is_active, false); // subscription should not be active even though its in the grace period since its cancelled

    let info = mock_info("charger", &[]);
    let msg = ExecuteMsg::Charge {
        payer_address: "subscriber".to_string(),
    };

    let res = contract::execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    match res {
        Err(ContractError::SubscriptionCancelled {}) => {}
        _ => panic!("Contract should return an error"),
    }
}

#[test]
fn remove_subscriber() {
    let mut deps = mock_dependencies(&[]);
    let msg = ProductInstantiateMsg {
        owner: "creator".to_string(),
        receiver_address: "receiver".to_string(),
        unit_amount: Uint256::from(123u128),
        initial_amount: Uint256::from(123u128),
        unit_interval_hour: 720u64,
        additional_grace_period_hour: Some(259200u64),
        max_amount_chargeable: Some(Uint256::from(123u128)),
        admins: Vec::new(),
        mutable: false,
        uri: "{\"image_url\": \"www.google.com\" }".to_string(),
        factory_address: "factory".to_string(),
    };

    let info = mock_info("creator", &[]);

    let _res = contract::instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let info_subscriber = mock_info("subscriber", &[]);

    let mut env = mock_env();
    let start_timestamp = 1609459200;
    env.block.time = Timestamp::from_seconds(start_timestamp); // set to 1 January 2021 00:00:00 GMT

    let msg = ExecuteMsg::Subscribe {};
    let res = contract::execute(
        deps.as_mut(),
        env.clone(),
        info_subscriber.clone(),
        msg.clone(),
    )
    .unwrap();

    // subscribe should be success
    assert_eq!(
        res.attributes,
        vec![
            attr("initial_amount", "123"),
            attr("method", "execute_subscribe"),
            attr("result", "subscribe_success"),
            attr("subscriber", "subscriber"),
            attr("module_contract_address", "cosmos2contract"),
        ]
    );

    // unauthorised user attempts to remove subscriber. Should fail
    let msg = ExecuteMsg::RemoveSubscriber {
        subscriber: "subscriber".to_string(),
    };

    let unauth_info = mock_info(
        "anyone",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );

    let res = contract::execute(deps.as_mut(), env.clone(), unauth_info.clone(), msg.clone());

    match res {
        Err(ContractError::Unauthorized {}) => {}
        _ => panic!("Must return unauthorized error"),
    }

    // attempts to remove subscriber which cannot be found, should return an error
    let invalid_msg = ExecuteMsg::RemoveSubscriber {
        subscriber: "carl".to_string(),
    };

    let res = contract::execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        invalid_msg.clone(),
    );

    match res {
        Err(ContractError::SubscriptionNotFound {}) => {}
        _ => panic!("Must return unauthorized error"),
    }

    let res = contract::execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("method", "execute_remove_subscriber"),
            attr("module_contract_address", "cosmos2contract")
        ]
    );

    let subscription: Option<SubscriptionInfoResponse> = from_binary(
        &contract::query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Subscription {
                subscriber: "subscriber".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(subscription, None);
}

#[test]
fn modify_subscriber() {
    let mut deps = mock_dependencies(&[]);
    let msg = ProductInstantiateMsg {
        owner: "creator".to_string(),
        receiver_address: "receiver".to_string(),
        unit_amount: Uint256::from(123u128),
        initial_amount: Uint256::from(123u128),
        unit_interval_hour: 720u64,
        additional_grace_period_hour: Some(259200u64),
        max_amount_chargeable: Some(Uint256::from(123u128)),
        admins: Vec::new(),
        mutable: false,
        uri: "{\"image_url\": \"www.google.com\" }".to_string(),
        factory_address: "factory".to_string(),
    };

    let info = mock_info("creator", &[]);

    let _res = contract::instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let mut env = mock_env();
    let start_timestamp = 1609459200;
    env.block.time = Timestamp::from_seconds(start_timestamp); // set to 1 January 2021 00:00:00 GMT

    let info_subscriber = mock_info("subscriber", &[]);
    let msg = ExecuteMsg::Subscribe {};

    let _res = contract::execute(
        deps.as_mut(),
        env.clone(),
        info_subscriber.clone(),
        msg.clone(),
    )
    .unwrap();

    // unauthorised user attempts to remove subscriber. Should fail
    let msg = ExecuteMsg::ModifySubscriber {
        new_created_at: None,
        new_last_charged: None,
        new_interval_end_at: None,
        subscriber: "subscriber".to_string(),
    };

    let unauth_info = mock_info(
        "anyone",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );

    let res = contract::execute(deps.as_mut(), env.clone(), unauth_info.clone(), msg.clone());

    match res {
        Err(ContractError::Unauthorized {}) => {}
        _ => panic!("Must return unauthorized error"),
    }

    let res = contract::execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("method", "modify_subscriber"),
            attr("module_contract_address", "cosmos2contract"),
        ]
    );

    // query subscriber
    let res = contract::query(
        deps.as_ref(),
        env.clone(),
        QueryMsg::Subscription {
            subscriber: "subscriber".to_string(),
        },
    )
    .unwrap();

    let subscriber_info: SubscriptionInfoResponse = from_binary(&res).unwrap();
    let expected_response: SubscriptionInfoResponse = SubscriptionInfoResponse {
        subscriber: "subscriber".to_string(),
        created_at: 1609459200,
        last_charged: 1609459200,
        interval_end_at: 1612051200,
        is_active: true,
        is_cancelled: false,
        discount_per_interval: None,
        amount_chargeable: Some(Uint256::zero()),
    };

    assert_eq!(subscriber_info, expected_response);
}

// tests if the discount is applied correctly
#[test]
fn test_discount() {
    let mut deps = mock_dependencies(&[]);
    let msg = ProductInstantiateMsg {
        owner: "creator".to_string(),
        receiver_address: "receiver".to_string(),
        unit_amount: Uint256::from(123u128),
        initial_amount: Uint256::from(123u128),
        unit_interval_hour: 720u64,
        additional_grace_period_hour: Some(259200u64),
        max_amount_chargeable: Some(Uint256::from(123u128)),
        admins: Vec::new(),
        mutable: false,
        uri: "{\"image_url\": \"www.google.com\" }".to_string(),
        factory_address: "factory".to_string(),
    };

    let info = mock_info("creator", &[]);

    let _res = contract::instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let mut env = mock_env();
    let start_timestamp = 1609459200;
    env.block.time = Timestamp::from_seconds(start_timestamp); // set to 1 January 2021 00:00:00 GMT

    let info_subscriber = mock_info("subscriber", &[]);
    let msg = ExecuteMsg::Subscribe {};

    let _res = contract::execute(
        deps.as_mut(),
        env.clone(),
        info_subscriber.clone(),
        msg.clone(),
    )
    .unwrap();

    // modify subscriber and give discount
    let msg = ExecuteMsg::SetDiscount {
        discount: Some(Discount {
            amount: Uint256::from(23u128),
        }),
        subscriber: "subscriber".to_string(),
    };

    // Subscriber cannot set discount
    contract::execute(
        deps.as_mut(),
        env.clone(),
        info_subscriber.clone(),
        msg.clone(),
    )
    .unwrap_err();

    // Only admin can set discount
    let res = contract::execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("method", "set_discount"),
            attr("module_contract_address", "cosmos2contract"),
        ]
    );

    // fast-forwards timestamp by one billing cycle and check that the subscription is still alive
    let info = mock_info("charger", &[]);
    env.block.time = Timestamp::from_seconds(start_timestamp + THIRTY_DAYS_IN_SECONDS);

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
            attr("amount", "100"),
            attr("periods", "1")
        ]
    );
}

#[test]
fn update_initial_amount_and_subscribe() {
    let mut deps = mock_dependencies(&[]);

    let msg = ProductInstantiateMsg {
        owner: "creator".to_string(),
        receiver_address: "receiver".to_string(),
        unit_amount: Uint256::from(123u128),
        initial_amount: Uint256::from(123u128),
        unit_interval_hour: 720u64,
        additional_grace_period_hour: Some(259200u64),
        max_amount_chargeable: Some(Uint256::from(123u128)),
        admins: Vec::new(),
        mutable: false,
        uri: "{\"image_url\": \"www.google.com\" }".to_string(),
        factory_address: "factory".to_string(),
    };

    let info = mock_info("creator", &[]);

    let mut env = mock_env();
    let start_timestamp = 1609459200;
    env.block.time = Timestamp::from_seconds(start_timestamp); // set to 1 January 2021 00:00:00 GMT

    let _res = contract::instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // update config for initial_amount
    let msg = ExecuteMsg::UpdateConfig {
        receiver_address: None,
        additional_grace_period_hour: None,
        initial_amount: Some(Uint256::from(100u128)), // change to 100
        uri: None,
    };

    let unauth_info = mock_info(
        "anyone",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    let res = contract::execute(deps.as_mut(), env.clone(), unauth_info.clone(), msg.clone());
    match res {
        Err(ContractError::Unauthorized {}) => {}
        _ => panic!("Must return unauthorized error"),
    }

    let res = contract::execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("method", "update_config"),
            attr("module_contract_address", "cosmos2contract"),
            attr("new_initial_amount", "100"),
        ]
    );

    // user tries to subscribe
    let msg = ExecuteMsg::Subscribe {};
    let info_subscriber = mock_info("subscriber", &[]);

    let res = contract::execute(
        deps.as_mut(),
        env.clone(),
        info_subscriber.clone(),
        msg.clone(),
    )
    .unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("initial_amount", "100"),
            attr("method", "execute_subscribe"),
            attr("result", "subscribe_success"),
            attr("subscriber", "subscriber"),
            attr("module_contract_address", "cosmos2contract"),
        ]
    );
}

#[test]
fn update_admins() {
    let mut deps = mock_dependencies(&[]);
    let msg = ProductInstantiateMsg {
        owner: "creator".to_string(),
        receiver_address: "receiver".to_string(),
        unit_amount: Uint256::from(123u128),
        initial_amount: Uint256::from(123u128),
        unit_interval_hour: 720u64,
        additional_grace_period_hour: Some(259200u64),
        max_amount_chargeable: Some(Uint256::from(123u128)),
        admins: Vec::new(),
        mutable: false,
        uri: "{\"image_url\": \"www.google.com\" }".to_string(),
        factory_address: "factory".to_string(),
    };

    let info = mock_info("creator", &[]);

    let _res = contract::instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let mut env = mock_env();
    let start_timestamp = 1609459200;
    env.block.time = Timestamp::from_seconds(start_timestamp); // set to 1 January 2021 00:00:00 GMT

    let admins: AdminConfigResponse = from_binary(
        &contract::query(deps.as_ref(), env.clone(), QueryMsg::AdminConfig {}).unwrap(),
    )
    .unwrap();

    // ensure expected config
    let mut expected = AdminConfigResponse {
        owner: "creator".to_string(),
        admins: vec![],
        mutable: false,
    };

    assert_eq!(admins, expected);

    let owner: Addr =
        from_binary(&contract::query(deps.as_ref(), env.clone(), QueryMsg::Owner {}).unwrap())
            .unwrap();
    assert_eq!(owner, Addr::unchecked("creator"));

    // update admins
    let msg = ExecuteMsg::UpdateAdmins {
        admins: vec!["alice".to_string()],
    };

    // contract cannot be updated by unauthorised personnels
    let unauth_info = mock_info(
        "anyone",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    let res = contract::execute(deps.as_mut(), mock_env(), unauth_info.clone(), msg.clone());
    match res {
        Err(ContractError::Unauthorized {}) => {}
        _ => panic!("Must return unauthorized error"),
    }

    let res2 = contract::execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // update should be successful
    assert_eq!(res2.attributes, vec![attr("action", "update_admins"),]);

    // add an additional admin
    let actual_response: AdminConfigResponse = from_binary(
        &contract::query(deps.as_ref(), env.clone(), QueryMsg::AdminConfig {}).unwrap(),
    )
    .unwrap();

    expected.admins.push("alice".to_string());

    assert_eq!(expected, actual_response);
}
#[test]
fn cancel_and_undo() {
    let mut deps = mock_dependencies(&[]);
    let msg = ProductInstantiateMsg {
        owner: "creator".to_string(),
        receiver_address: "receiver".to_string(),
        unit_amount: Uint256::from(123u128),
        initial_amount: Uint256::from(123u128),
        unit_interval_hour: 720u64,
        additional_grace_period_hour: Some(259200u64),
        max_amount_chargeable: Some(Uint256::from(123u128)),
        admins: Vec::new(),
        mutable: false,
        uri: "{\"image_url\": \"www.google.com\" }".to_string(),
        factory_address: "factory".to_string(),
    };

    let info = mock_info("creator", &[]);

    let _res = contract::instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let info_subscriber = mock_info("subscriber", &[]);

    let mut env = mock_env();
    let start_timestamp = 1609459200;
    env.block.time = Timestamp::from_seconds(start_timestamp); // set to 1 January 2021 00:00:00 GMT

    let msg = ExecuteMsg::Subscribe {};
    let res = contract::execute(
        deps.as_mut(),
        env.clone(),
        info_subscriber.clone(),
        msg.clone(),
    )
    .unwrap();

    // subscribe should be success
    assert_eq!(
        res.attributes,
        vec![
            attr("initial_amount", "123"),
            attr("method", "execute_subscribe"),
            attr("result", "subscribe_success"),
            attr("subscriber", "subscriber"),
            attr("module_contract_address", "cosmos2contract"),
        ]
    );

    // subscriber attempts to cancel. This should succeed
    let msg = ExecuteMsg::Cancel {};
    let res = contract::execute(
        deps.as_mut(),
        env.clone(),
        info_subscriber.clone(),
        msg.clone(),
    )
    .unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("method", "execute_cancel"),
            attr("module_contract_address", "cosmos2contract")
        ]
    );

    let get_subscription: Option<SubscriptionInfoResponse> = from_binary(
        &contract::query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Subscription {
                subscriber: "subscriber".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();

    match get_subscription {
        Some(subscription) => {
            // subscription should be cancelled
            assert_eq!(subscription.is_cancelled, true);
            assert_eq!(subscription.is_active, true); // subscription should still be active even though it is cancelled
        }
        None => panic!("Contract should return an error"),
    };

    let new_timestamp = start_timestamp + 200 * 60 * 60;
    env.block.time = Timestamp::from_seconds(new_timestamp); // set to 1 January 2021 00:00:00 GMT

    let msg = ExecuteMsg::Subscribe {};
    let res = contract::execute(
        deps.as_mut(),
        env.clone(),
        info_subscriber.clone(),
        msg.clone(),
    )
    .unwrap();

    // subscribe should be success
    assert_eq!(
        res.attributes,
        vec![
            attr("additional_info", "undo_cancellation"),
            attr("method", "execute_subscribe"),
            attr("result", "subscribe_success"),
            attr("subscriber", "subscriber"),
            attr("module_contract_address", "cosmos2contract"),
        ]
    );

    let subscription: SubscriptionInfoResponse = from_binary(
        &contract::query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Subscription {
                subscriber: "subscriber".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();

    let expected_subscription = SubscriptionInfoResponse {
        subscriber: "subscriber".to_string(),
        created_at: start_timestamp,
        last_charged: start_timestamp,
        interval_end_at: start_timestamp + THIRTY_DAYS_IN_SECONDS,
        is_cancelled: false,
        is_active: true,
        discount_per_interval: None,
        amount_chargeable: Some(Uint256::zero()),
    };

    // subscription should be cancelled
    assert_eq!(subscription, expected_subscription);
}

#[test]
fn test_compute_amount_chargeable() {
    let deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(100u128),
    }]);
    // Charge $1 per minute
    let state = Config {
        owner_address: Addr::unchecked("owner"),
        receiver_address: Addr::unchecked("receiver"),
        additional_grace_period: 0,
        paused: false,
        unit_amount: Uint256::from(1000000u128),
        initial_amount: Uint256::from(1000000u128),
        unit_interval: Timestamp::from_seconds(60u64),
        uri: "{\"image_url\": \"www.google.com\" }".to_string(),
        factory_address: Addr::unchecked("factory"),
    };

    let subscription_info = SubscriptionInfo {
        owner: deps.api.addr_validate("suberra").unwrap(),
        created_at: Timestamp::from_seconds(100000),
        last_charged: Timestamp::from_seconds(100000),
        interval_end_at: Timestamp::from_seconds(100060),
        is_cancelled: false,
        discount_per_interval: None,
    };
    // charge after 10mins
    let amount_chargeable = contract::compute_amount_chargeable(
        &state,
        &subscription_info,
        Timestamp::from_seconds(100659),
    );
    assert_eq!(amount_chargeable.amount, Uint256::from(10_000_000u128));

    // Charge $1 every 5 minutes
    let state = Config {
        owner_address: Addr::unchecked("owner"),
        receiver_address: Addr::unchecked("receiver"),
        paused: false,
        unit_amount: Uint256::from(1000000u128),
        initial_amount: Uint256::from(1000000u128),
        unit_interval: Timestamp::from_seconds(300u64),
        additional_grace_period: 0,
        uri: "{\"image_url\": \"www.google.com\" }".to_string(),
        factory_address: Addr::unchecked("factory"),
    };

    let subscription_info = SubscriptionInfo {
        owner: deps.api.addr_validate("suberra").unwrap(),
        created_at: Timestamp::from_seconds(100000),
        last_charged: Timestamp::from_seconds(100000),
        interval_end_at: Timestamp::from_seconds(100300),
        is_cancelled: false,
        discount_per_interval: None,
    };
    // charge $0 after 1min
    let amount_chargeable = contract::compute_amount_chargeable(
        &state,
        &subscription_info,
        Timestamp::from_seconds(100060),
    );
    assert_eq!(amount_chargeable.amount, Uint256::from(0u128));

    // charge $1 after 5min
    let amount_chargeable = contract::compute_amount_chargeable(
        &state,
        &subscription_info,
        Timestamp::from_seconds(100300),
    );
    assert_eq!(amount_chargeable.amount, Uint256::from(1_000_000u128));

    // charge $2 after 11min
    let amount_chargeable = contract::compute_amount_chargeable(
        &state,
        &subscription_info,
        Timestamp::from_seconds(100661),
    );
    assert_eq!(amount_chargeable.amount, Uint256::from(2_000_000u128));
}
