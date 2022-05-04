use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::mock_querier::mock_dependencies;
use crate::msg::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg, StreamsResponse};
use crate::state::Stream;
use crate::token::{Asset, AssetInfo};

use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{
    attr, coins, from_binary, to_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, StdError, SubMsg,
    Timestamp, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

const HOUR_SECONDS: u64 = 3600u64;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {};
    let info = mock_info("creator", &coins(1000, "earth"));

    // we can just call .unwrap() to assert this was a success
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn create_native_stream() {
    let start_time = 1000u64;
    let mut deps = mock_dependencies(&[]);
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(start_time);

    let msg = InstantiateMsg {};
    let info = mock_info("creator", &[]);
    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Create native stream,
    // Alice -> Bob, $1000 in 1000s starts immediately
    let msg = ExecuteMsg::CreateStream {
        receiver: "bob".to_string(),
        token: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::from(1000_000_000u128),
        },
        start_at: None,
        end_at: start_time + 1000,
    };

    // Error when no funds are sent
    let info = mock_info("alice", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap_err();
    match res {
        ContractError::Std(StdError::GenericErr { msg, .. }) => assert_eq!(
            msg,
            "Native token balance mismatch between the argument and the transferred".to_string()
        ),
        _ => panic!("Must return generic error"),
    }

    // Create success
    let info = mock_info("alice", &coins(1000_000_000, "uusd"));
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("method", "create_stream"),
            attr("stream_id", 1.to_string())
        ]
    );

    // Cannot create stream with invalid start_at
    let msg = ExecuteMsg::CreateStream {
        receiver: "bob".to_string(),
        token: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::from(999_000_001u128),
        },
        start_at: Some(start_time - 10),
        end_at: start_time + HOUR_SECONDS + 1000,
    };
    let info = mock_info("alice", &coins(999_000_001, "uusd"));
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap_err();
    match res {
        ContractError::InvalidParam { name, message: _ } => {
            assert_eq!(name, "start_at")
        }
        _ => panic!("Must return error"),
    }

    // Cannot create stream with invalid end_at
    let msg = ExecuteMsg::CreateStream {
        receiver: "bob".to_string(),
        token: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::from(999_000_001u128),
        },
        start_at: Some(start_time + HOUR_SECONDS),
        end_at: start_time + HOUR_SECONDS - 1,
    };
    let info = mock_info("alice", &coins(999_000_001, "uusd"));
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap_err();
    match res {
        ContractError::InvalidParam { name, message: _ } => {
            assert_eq!(name, "end_at")
        }
        _ => panic!("Must return error"),
    }

    // Cannot create stream with 0 duration
    let msg = ExecuteMsg::CreateStream {
        receiver: "bob".to_string(),
        token: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::from(999_000_001u128),
        },
        start_at: None,
        end_at: start_time,
    };
    let info = mock_info("alice", &coins(999_000_001, "uusd"));
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap_err();
    match res {
        ContractError::InvalidParam { name, message: _ } => {
            assert_eq!(name, "end_at")
        }
        _ => panic!("Must return error"),
    }

    // Cannot create stream with invalid amount
    let msg = ExecuteMsg::CreateStream {
        receiver: "bob".to_string(),
        token: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::from(0u128),
        },
        start_at: Some(start_time + HOUR_SECONDS),
        end_at: start_time + HOUR_SECONDS + 1000,
    };
    let info = mock_info("alice", &coins(999_000_001, "uusd"));
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap_err();
    match res {
        ContractError::InvalidParam { name, message: _ } => {
            assert_eq!(name, "amount")
        }
        _ => panic!("Must return error"),
    }

    // Cannot create stream to self
    let msg = ExecuteMsg::CreateStream {
        receiver: "alice".to_string(),
        token: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::from(0u128),
        },
        start_at: Some(start_time + HOUR_SECONDS),
        end_at: start_time + HOUR_SECONDS + 1000,
    };
    let info = mock_info("alice", &coins(999_000_001, "uusd"));
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap_err();
    match res {
        ContractError::InvalidParam { name, message: _ } => {
            assert_eq!(name, "receiver")
        }
        _ => panic!("Must return error"),
    }

    // Cannot create stream to contract itself
    let msg = ExecuteMsg::CreateStream {
        receiver: "cosmos2contract".to_string(),
        token: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::from(0u128),
        },
        start_at: Some(start_time + HOUR_SECONDS),
        end_at: start_time + HOUR_SECONDS + 1000,
    };
    let info = mock_info("alice", &coins(999_000_001, "uusd"));
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap_err();
    match res {
        ContractError::InvalidParam { name, message: _ } => {
            assert_eq!(name, "receiver")
        }
        _ => panic!("Must return error"),
    }

    // Can create stream with non-multiples of duration
    // Alice -> Bob, $999.000001 in 1000s
    let msg = ExecuteMsg::CreateStream {
        receiver: "bob".to_string(),
        token: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::from(999_000_001u128),
        },
        start_at: Some(start_time + HOUR_SECONDS),
        end_at: start_time + HOUR_SECONDS + 1000,
    };
    let info = mock_info("alice", &coins(999_000_001, "uusd"));
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("method", "create_stream"),
            attr("stream_id", 2.to_string())
        ]
    );
}

#[test]
fn create_token_stream() {
    let start_time = 1000u64;
    let mut deps = mock_dependencies(&[]);
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(start_time);

    let msg = InstantiateMsg {};
    let info = mock_info("creator", &[]);
    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Error when sending Invalid hook message
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "alice".to_string(),
        msg: to_binary(&ExecuteMsg::Withdraw {
            stream_id: 1u64,
            amount: None,
        })
        .unwrap(),
        amount: Uint128::from(1100_000_000u128),
    });

    let info = mock_info("cw20-token-a", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap_err();
    match res {
        ContractError::Std(_) => {}
        _ => panic!("Must return parse error {:?}", res),
    }

    // Error when sending from wrong token amount
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "alice".to_string(),
        msg: to_binary(&Cw20HookMsg::CreateStream {
            receiver: "bob".to_string(),
            token: Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("cw20-token-a"),
                },
                amount: Uint128::from(1000_000_000u128),
            },
            start_at: Some(start_time + HOUR_SECONDS),
            end_at: start_time + HOUR_SECONDS + 1000,
        })
        .unwrap(),
        // Sent more token
        amount: Uint128::from(1100_000_000u128),
    });

    let info = mock_info("cw20-token-a", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap_err();
    match res {
        ContractError::TokenMismatch {} => {}
        _ => panic!("Must return token mismatch error"),
    }

    // Error when sending wrong token
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "alice".to_string(),
        msg: to_binary(&Cw20HookMsg::CreateStream {
            receiver: "bob".to_string(),
            token: Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("cw20-token-a"),
                },
                amount: Uint128::from(1000_000_000u128),
            },
            start_at: Some(start_time + HOUR_SECONDS),
            end_at: start_time + HOUR_SECONDS + 1000,
        })
        .unwrap(),
        amount: Uint128::from(1000_000_000u128),
    });

    let info = mock_info("cw20-token-b", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap_err();
    match res {
        ContractError::TokenMismatch {} => {}
        _ => panic!("Must return token mismatch error"),
    }

    // Create cw20 token stream
    // Alice -> Bob, $1000 in 1000s
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "alice".to_string(),
        msg: to_binary(&Cw20HookMsg::CreateStream {
            receiver: "bob".to_string(),
            token: Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("cw20-token-a"),
                },
                amount: Uint128::from(1000_000_000u128),
            },
            start_at: None,
            end_at: start_time + 1000,
        })
        .unwrap(),
        amount: Uint128::from(1000_000_000u128),
    });

    // Error when sending from wrong token address
    let info = mock_info("cw20-token-b", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap_err();
    match res {
        ContractError::TokenMismatch {} => {}
        _ => panic!("Must return token mismatch error"),
    }

    // Create success
    let info = mock_info("cw20-token-a", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("method", "create_stream"),
            attr("stream_id", 1.to_string())
        ]
    );
}

#[test]
fn withdraw_streams() {
    let start_time = 1000u64;
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::zero(),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(start_time);

    let msg = InstantiateMsg {};
    let info = mock_info("creator", &[]);
    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Create cw20 token stream
    // Alice -> Bob, $1000 in 1000s, $1/s
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "alice".to_string(),
        msg: to_binary(&Cw20HookMsg::CreateStream {
            receiver: "bob".to_string(),
            token: Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("cw20-token-a"),
                },
                amount: Uint128::from(1000_000_001u128),
            },
            start_at: Some(start_time + HOUR_SECONDS),
            end_at: start_time + HOUR_SECONDS + 1000,
        })
        .unwrap(),
        amount: Uint128::from(1000_000_001u128),
    });

    // Create success
    let info = mock_info("cw20-token-a", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("method", "create_stream"),
            attr("stream_id", 1.to_string())
        ]
    );

    env.block.time = Timestamp::from_seconds(start_time + HOUR_SECONDS);

    // Bob cannot withdraw 0 amount
    let msg = ExecuteMsg::Withdraw {
        stream_id: 1u64,
        amount: None,
    };

    let info = mock_info("bob", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
    match res {
        ContractError::ZeroTransferableAmount {} => {}
        _ => panic!("Must return insufficient error, got {:?}", res),
    }

    // Bob cannot withdraw from non-existant stream
    let msg = ExecuteMsg::Withdraw {
        stream_id: 2u64,
        amount: None,
    };

    let info = mock_info("bob", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
    match res {
        ContractError::StreamNotFound {} => {}
        _ => panic!("Must return StreamNotFound error, got {:?}", res),
    }

    // 50% streamed
    env.block.time = Timestamp::from_seconds(start_time + HOUR_SECONDS + 500);

    // Charlie can help withdraw/settle alice-bob's stream
    let msg = ExecuteMsg::Withdraw {
        stream_id: 1u64,
        amount: Some(Uint128::from(1u64)),
    };

    let info = mock_info("charlie", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "cw20-token-a".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "bob".to_string(),
                amount: Uint128::from(1u128)
            })
            .unwrap(),
            funds: vec![],
        }))]
    );

    // Bob cannot withdraw more than streamed
    let msg = ExecuteMsg::Withdraw {
        stream_id: 1u64,
        amount: Some(Uint128::from(500_000_001u128)),
    };

    let info = mock_info("bob", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
    match res {
        ContractError::InsufficientBalance {} => {}
        _ => panic!("Must return insufficient error, got {:?}", res),
    }

    // Bob withdraws some
    let msg = ExecuteMsg::Withdraw {
        stream_id: 1u64,
        amount: Some(Uint128::from(200_000_000u128)),
    };

    let info = mock_info("bob", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "cw20-token-a".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "bob".to_string(),
                amount: Uint128::from(200_000_000u128)
            })
            .unwrap(),
            funds: vec![],
        }))]
    );

    // Alice can withdraw for bob
    let msg = ExecuteMsg::Withdraw {
        stream_id: 1u64,
        amount: None,
    };

    let info = mock_info("alice", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "cw20-token-a".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "bob".to_string(),
                amount: Uint128::from(299_999_999u128)
            })
            .unwrap(),
            funds: vec![],
        }))]
    );

    // 100% streamed
    env.block.time = Timestamp::from_seconds(start_time + HOUR_SECONDS + 1000);

    // Bob withdraws remaining amount
    let msg = ExecuteMsg::Withdraw {
        stream_id: 1u64,
        amount: Some(Uint128::from(500_000_001u128)),
    };

    let info = mock_info("bob", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "cw20-token-a".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "bob".to_string(),
                amount: Uint128::from(500_000_001u128)
            })
            .unwrap(),
            funds: vec![],
        }))]
    );

    env.block.time = Timestamp::from_seconds(start_time + HOUR_SECONDS + 2000);
    // Create native stream
    // Charlie -> Bob, $1000 in 100s, $10/s
    let msg = ExecuteMsg::CreateStream {
        receiver: "bob".to_string(),
        token: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::from(1000_000_000u128),
        },
        start_at: Some(start_time + HOUR_SECONDS + 2000),
        end_at: start_time + HOUR_SECONDS + 2100,
    };
    let info = mock_info("charlie", &coins(1000_000_000u128, "uusd"));
    execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    env.block.time = Timestamp::from_seconds(start_time + HOUR_SECONDS + 2050);

    // Bob withdraws halfway through the stream
    let msg = ExecuteMsg::Withdraw {
        stream_id: 2u64,
        amount: None,
    };

    let info = mock_info("bob", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "bob".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(500_000_000u128)
            }],
        }))]
    );
}

#[test]
fn cancel_streams() {
    let start_time = 1000u64;
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::zero(),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(start_time);

    let msg = InstantiateMsg {};
    let info = mock_info("creator", &[]);
    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Create native stream
    // Alice -> Bob, $1000 in 2000s, $0.5/s
    let msg = ExecuteMsg::CreateStream {
        receiver: "bob".to_string(),
        token: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::from(1000_000_000u128),
        },
        start_at: Some(start_time),
        end_at: start_time + 2000,
    };

    // Create success
    let info = mock_info("alice", &coins(1000_000_000u128, "uusd"));
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("method", "create_stream"),
            attr("stream_id", 1.to_string())
        ]
    );

    // 1/4 time has passed, $250 streamed
    env.block.time = Timestamp::from_seconds(start_time + 500);

    // Charlie cannot cancel alice-bob's stream
    let msg = ExecuteMsg::CancelStream { stream_id: 1u64 };

    let info = mock_info("charlie", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
    match res {
        ContractError::Unauthorized {} => {}
        _ => panic!("Must return authorized error, got {:?}", res),
    }

    // Bob can cancel alice-bob's stream
    let msg = ExecuteMsg::CancelStream { stream_id: 1u64 };

    let info = mock_info("bob", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Transfers streamed & unstreamed amount
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "bob".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(250_000_000u128)
                }],
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "alice".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(750_000_000u128)
                },],
            })),
        ]
    );
}

// Queries
#[test]
fn query_streams() {
    let start_time = 1000u64;
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::zero(),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(start_time);

    let msg = InstantiateMsg {};
    let info = mock_info("creator", &[]);
    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Create cw20 token stream
    // #1. Alice -> Bob, $1000 in 1000s, $1/s
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "alice".to_string(),
        msg: to_binary(&Cw20HookMsg::CreateStream {
            receiver: "bob".to_string(),
            token: Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("cw20-token-a"),
                },
                amount: Uint128::from(1000_000_000u128),
            },
            start_at: Some(start_time),
            end_at: start_time + 1000,
        })
        .unwrap(),
        amount: Uint128::from(1000_000_000u128),
    });
    let info = mock_info("cw20-token-a", &[]);
    execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Create native stream
    // #2. Bob -> Alice, $1000 in 2000s, $0.5/s
    let msg = ExecuteMsg::CreateStream {
        receiver: "alice".to_string(),
        token: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::from(1000_000_000u128),
        },
        start_at: Some(start_time),
        end_at: start_time + 2000,
    };

    let info = mock_info("bob", &coins(1000_000_000u128, "uusd"));
    execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Create native stream
    // #3. Charlie -> Alice, $2000 in 200s, $10/s
    let msg = ExecuteMsg::CreateStream {
        receiver: "alice".to_string(),
        token: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::from(2000_000_000u128),
        },
        start_at: Some(start_time),
        end_at: start_time + 200,
    };

    let info = mock_info("charlie", &coins(2000_000_000u128, "uusd"));
    execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Get all streams
    assert_eq!(
        from_binary::<StreamsResponse>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::AllStreams {
                    start_after: None,
                    limit: None
                },
            )
            .unwrap()
        )
        .unwrap(),
        StreamsResponse {
            stream_ids: vec![1, 2, 3],
            last_key: Some(3)
        }
    );

    // Get sender streams
    assert_eq!(
        from_binary::<StreamsResponse>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::StreamsBySender {
                    sender: "alice".to_string(),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap()
        )
        .unwrap(),
        StreamsResponse {
            stream_ids: vec![1],
            last_key: Some(1)
        }
    );

    // Get receiver streams
    assert_eq!(
        from_binary::<StreamsResponse>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::StreamsByReceiver {
                    receiver: "alice".to_string(),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap()
        )
        .unwrap(),
        StreamsResponse {
            stream_ids: vec![2, 3],
            last_key: Some(3)
        }
    );

    // Get native token streams
    assert_eq!(
        from_binary::<StreamsResponse>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::StreamsByToken {
                    token_info: AssetInfo::NativeToken {
                        denom: "uusd".to_string()
                    },
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap()
        )
        .unwrap(),
        StreamsResponse {
            stream_ids: vec![2, 3],
            last_key: Some(3)
        }
    );

    // Get cw20 token streams
    assert_eq!(
        from_binary::<StreamsResponse>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::StreamsByToken {
                    token_info: AssetInfo::Token {
                        contract_addr: Addr::unchecked("cw20-token-a")
                    },
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap()
        )
        .unwrap(),
        StreamsResponse {
            stream_ids: vec![1],
            last_key: Some(1)
        }
    );
}

#[test]
fn query_balance() {
    let start_time = 1000u64;
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::zero(),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(start_time);

    let msg = InstantiateMsg {};
    let info = mock_info("creator", &[]);
    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Create cw20 token stream
    // #1. Alice -> Bob, $1000 in 1000s, $1/s
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "alice".to_string(),
        msg: to_binary(&Cw20HookMsg::CreateStream {
            receiver: "bob".to_string(),
            token: Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("cw20-token-a"),
                },
                amount: Uint128::from(1000_000_000u128),
            },
            start_at: Some(start_time),
            end_at: start_time + 1000,
        })
        .unwrap(),
        amount: Uint128::from(1000_000_000u128),
    });
    let info = mock_info("cw20-token-a", &[]);
    execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // 10% passed, $100 streamed
    env.block.time = Timestamp::from_seconds(start_time + 100);

    assert_eq!(
        from_binary::<Uint128>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::BalanceOf {
                    stream_id: 1u64,
                    address: "alice".to_string(),
                },
            )
            .unwrap()
        )
        .unwrap(),
        Uint128::from(900_000_000u128)
    );

    assert_eq!(
        from_binary::<Uint128>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::BalanceOf {
                    stream_id: 1u64,
                    address: "bob".to_string(),
                },
            )
            .unwrap()
        )
        .unwrap(),
        Uint128::from(100_000_000u128)
    );

    // Query balance of neither sender or receiver should return 0
    assert_eq!(
        from_binary::<Uint128>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::BalanceOf {
                    stream_id: 1u64,
                    address: "charlie".to_string(),
                },
            )
            .unwrap()
        )
        .unwrap(),
        Uint128::zero()
    );

    // >100% passed, $1000 streamed
    env.block.time = Timestamp::from_seconds(start_time + 2000);

    assert_eq!(
        from_binary::<Uint128>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::BalanceOf {
                    stream_id: 1u64,
                    address: "alice".to_string(),
                },
            )
            .unwrap()
        )
        .unwrap(),
        Uint128::zero()
    );

    assert_eq!(
        from_binary::<Uint128>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::BalanceOf {
                    stream_id: 1u64,
                    address: "bob".to_string(),
                },
            )
            .unwrap()
        )
        .unwrap(),
        Uint128::from(1000_000_000u128)
    );

    // Create native token stream
    // #2. Alice -> Charlie, $1000.0001 in 100s, $10+/s
    let msg = ExecuteMsg::CreateStream {
        receiver: "charlie".to_string(),
        token: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::from(1000_000_100u128),
        },
        start_at: Some(start_time + 2000),
        end_at: start_time + 2000 + 100,
    };

    let info = mock_info("alice", &coins(1000_000_100u128, "uusd"));
    execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // 10% passed, $100+ streamed
    env.block.time = Timestamp::from_seconds(start_time + 2000 + 10);

    assert_eq!(
        from_binary::<Uint128>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::BalanceOf {
                    stream_id: 2u64,
                    address: "alice".to_string(),
                },
            )
            .unwrap()
        )
        .unwrap(),
        Uint128::from(900_000_090u128)
    );

    assert_eq!(
        from_binary::<Uint128>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::BalanceOf {
                    stream_id: 2u64,
                    address: "charlie".to_string(),
                },
            )
            .unwrap()
        )
        .unwrap(),
        Uint128::from(100_000_010u128)
    );

    // 33% passed, $300+ streamed
    env.block.time = Timestamp::from_seconds(start_time + 2000 + 33);

    assert_eq!(
        from_binary::<Uint128>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::BalanceOf {
                    stream_id: 2u64,
                    address: "alice".to_string(),
                },
            )
            .unwrap()
        )
        .unwrap(),
        Uint128::from(670_000_067u128)
    );

    assert_eq!(
        from_binary::<Uint128>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::BalanceOf {
                    stream_id: 2u64,
                    address: "charlie".to_string(),
                },
            )
            .unwrap()
        )
        .unwrap(),
        Uint128::from(330_000_033u128)
    );

    // Withdraw current balance
    let info = mock_info("charlie", &[]);
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::Withdraw {
            stream_id: 2u64,
            amount: None,
        },
    )
    .unwrap();

    assert_eq!(
        from_binary::<Uint128>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::BalanceOf {
                    stream_id: 2u64,
                    address: "charlie".to_string(),
                },
            )
            .unwrap()
        )
        .unwrap(),
        Uint128::zero()
    );

    // 99% passed, $990+ streamed
    env.block.time = Timestamp::from_seconds(start_time + 2000 + 99);

    assert_eq!(
        from_binary::<Uint128>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::BalanceOf {
                    stream_id: 2u64,
                    address: "alice".to_string(),
                },
            )
            .unwrap()
        )
        .unwrap(),
        Uint128::from(10_000_001u128)
    );

    assert_eq!(
        from_binary::<Uint128>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::BalanceOf {
                    stream_id: 2u64,
                    address: "charlie".to_string(),
                },
            )
            .unwrap()
        )
        .unwrap(),
        Uint128::from(660_000_066u128)
    );

    // 100% passed, $1000+ streamed
    env.block.time = Timestamp::from_seconds(start_time + 2000 + 100);

    assert_eq!(
        from_binary::<Uint128>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::BalanceOf {
                    stream_id: 2u64,
                    address: "alice".to_string(),
                },
            )
            .unwrap()
        )
        .unwrap(),
        Uint128::zero()
    );

    assert_eq!(
        from_binary::<Uint128>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::BalanceOf {
                    stream_id: 2u64,
                    address: "charlie".to_string(),
                },
            )
            .unwrap()
        )
        .unwrap(),
        Uint128::from(670_000_067u128)
    );
}

#[test]
fn query_stream() {
    let start_time = 1000u64;
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::zero(),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(start_time);

    let msg = InstantiateMsg {};
    let info = mock_info("creator", &[]);
    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Create cw20 token stream
    // #1. Alice -> Bob, $1000 in 1000s, $1/s
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "alice".to_string(),
        msg: to_binary(&Cw20HookMsg::CreateStream {
            receiver: "bob".to_string(),
            token: Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("cw20-token-a"),
                },
                amount: Uint128::from(1000_000_067u128),
            },
            start_at: Some(start_time),
            end_at: start_time + 1000,
        })
        .unwrap(),
        amount: Uint128::from(1000_000_067u128),
    });
    let info = mock_info("cw20-token-a", &[]);
    execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    assert_eq!(
        from_binary::<Stream>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::Stream { stream_id: 1u64 },
            )
            .unwrap()
        )
        .unwrap(),
        Stream {
            sender: Addr::unchecked("alice"),
            receiver: Addr::unchecked("bob"),
            token: Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("cw20-token-a"),
                },
                amount: Uint128::from(1000_000_067u128),
            },
            rate_per_second: Decimal::from_ratio(1000_000_067u128, 1000u128),
            remaining_amount: Uint128::from(1000_000_067u128),
            start_at: start_time,
            end_at: start_time + 1000,
        }
    );

    env.block.time = Timestamp::from_seconds(start_time + 900);

    // Claim 90% amount
    let info = mock_info("bob", &[]);
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::Withdraw {
            stream_id: 1u64,
            amount: None,
        },
    )
    .unwrap();

    assert_eq!(
        from_binary::<Stream>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::Stream { stream_id: 1u64 },
            )
            .unwrap()
        )
        .unwrap(),
        Stream {
            sender: Addr::unchecked("alice"),
            receiver: Addr::unchecked("bob"),
            token: Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("cw20-token-a"),
                },
                amount: Uint128::from(1000_000_067u128),
            },
            rate_per_second: Decimal::from_ratio(1000_000_067u128, 1000u128),
            remaining_amount: Uint128::from(100_000_007u128),
            start_at: start_time,
            end_at: start_time + 1000,
        }
    );

    env.block.time = Timestamp::from_seconds(start_time + 1000);

    // Claim 100% amount
    let info = mock_info("bob", &[]);
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::Withdraw {
            stream_id: 1u64,
            amount: None,
        },
    )
    .unwrap();

    // Stream deleted
    assert_eq!(
        query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Stream { stream_id: 1u64 },
        )
        .unwrap_err(),
        StdError::NotFound {
            kind: "token_stream::state::Stream".to_string()
        }
    );
}
