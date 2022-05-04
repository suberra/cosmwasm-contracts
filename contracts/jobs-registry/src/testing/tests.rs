use crate::contract::execute;
use crate::contract::instantiate;
use crate::contract::query;
use crate::error::ContractError;
use crate::msg::AllJobsResponse;
use crate::msg::ConfigResponse;
use crate::msg::ExecuteMsg;
use crate::msg::InstantiateMsg;
use crate::msg::JobInfo;
use crate::msg::QueryMsg;
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::Coin;
use cosmwasm_std::Decimal;
use cosmwasm_std::Uint128;
use cosmwasm_std::{coins, from_binary};
use cw0::NativeBalance;

use crate::testing::mock_querier::mock_dependencies;
#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {};
    let info = mock_info("creator", &[]);

    // we can just call .unwrap() to assert this was a success
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetConfig {}).unwrap();
    let value: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!("creator", value.owner);
}

#[test]
fn add_job() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {};
    let info = mock_info("creator", &[]);

    let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());

    let msg = ExecuteMsg::UpdateAdmins {
        admins: vec!["merchant".to_owned(), "merchant2".to_owned()],
    };

    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let info = mock_info("merchant", &coins(2, "token"));
    let msg = ExecuteMsg::AddJob {
        contract_address: String::from("job1"),
        name: String::from("TestJob1"),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    for attr in res.attributes.iter() {
        println!("{}: {}", attr.key, attr.value)
    }
    assert_eq!(3, res.attributes.len());

    // should not be able to add same job
    let info = mock_info("merchant2", &coins(2, "token"));
    let msg = ExecuteMsg::AddJob {
        contract_address: String::from("job1"),
        name: String::from("TestJob1"),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(ContractError::JobExist {}) => {}
        _ => panic!("Should not be able to add same job contract"),
    }

    let info = mock_info("merchant", &coins(2, "token"));
    let msg = ExecuteMsg::AddJob {
        contract_address: String::from("job2"),
        name: String::from("TestJob2"),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    for attr in res.attributes.iter() {
        println!("{}: {}", attr.key, attr.value)
    }
    assert_eq!("job2", res.attributes[1].value);
    assert_eq!("2", res.attributes[2].value);
}

#[test]
fn remove_job() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {};
    let info = mock_info("creator", &[]);

    let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());

    let msg = ExecuteMsg::UpdateAdmins {
        admins: vec!["merchant".to_owned()],
    };

    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // should not be able to remove non-existent jobs
    let info = mock_info("merchant", &coins(2, "token"));
    let msg = ExecuteMsg::RemoveJob {
        contract_address: String::from("job1"),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(ContractError::JobNotFound {}) => {}
        _ => panic!("Should not be able to remove non-existent jobs"),
    }

    let info = mock_info("merchant", &coins(2, "token"));
    let msg = ExecuteMsg::AddJob {
        contract_address: String::from("job1"),
        name: String::from("TestJob1"),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(3, res.attributes.len());

    let info = mock_info("merchant", &coins(2, "token"));
    let msg = ExecuteMsg::RemoveJob {
        contract_address: String::from("job1"),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!("job1", res.attributes[1].value);
}

#[test]
fn get_job() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {};
    let info = mock_info("creator", &[]);

    let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());

    let msg = ExecuteMsg::UpdateAdmins {
        admins: vec!["merchant".to_owned()],
    };

    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let info = mock_info("merchant", &coins(2, "token"));
    let msg = ExecuteMsg::AddJob {
        contract_address: String::from("job1"),
        name: String::from("TestJob1"),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(3, res.attributes.len());

    let msg = QueryMsg::GetJob {
        contract_address: String::from("job1"),
    };
    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let value: JobInfo = from_binary(&res).unwrap();
    assert_eq!("job1", value.contract_address);
    assert!(value.is_active);
    assert_eq!("merchant", value.owner);
    assert_eq!(1, value.job_id);
}

#[test]
fn get_all_jobs() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {};
    let info = mock_info("creator", &[]);

    let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());

    let msg = ExecuteMsg::UpdateAdmins {
        admins: vec!["merchant".to_owned()],
    };

    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    for i in 0..30 {
        let job_contract_addr = format!("job{}", i);
        let job_contract_name = format!("TestJob{}", i);
        let info = mock_info("merchant", &coins(2, "token"));
        let msg = ExecuteMsg::AddJob {
            contract_address: job_contract_addr,
            name: job_contract_name,
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    let msg = QueryMsg::AllJobs {
        start_after: None,
        limit: None,
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let value: AllJobsResponse = from_binary(&res).unwrap();
    assert_eq!(10, value.jobs.len());
    for job in value.jobs.iter() {
        println!("{}", job.contract_address);
        assert!(job.is_active);
    }

    let last_job_address = value.jobs.last().unwrap().contract_address.clone();

    let msg = QueryMsg::AllJobs {
        start_after: Some(last_job_address.to_string()),
        limit: Some(20),
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let value: AllJobsResponse = from_binary(&res).unwrap();
    assert_eq!(20, value.jobs.len());
    for job in value.jobs.iter() {
        println!("{}", job.contract_address);
        println!("{}", job.name);
        assert!(job.is_active);
    }
}

#[test]
fn add_credits() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {};
    let info = mock_info("creator", &[]);

    let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());

    let msg = ExecuteMsg::UpdateAdmins {
        admins: vec!["merchant".to_owned()],
    };

    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::SetBaseFee {
        base_fee: vec![Coin::new(100000, "uusd")],
    };

    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let info = mock_info("merchant", &coins(5, "uusd"));
    let msg = ExecuteMsg::AddCredits {
        contract_address: String::from("job1"),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(ContractError::JobNotFound {}) => {}
        _ => panic!("Should not be able to add credit to non-existent jobs"),
    }

    let info = mock_info("merchant", &vec![]);
    let msg = ExecuteMsg::AddJob {
        contract_address: String::from("job1"),
        name: String::from("TestJob1"),
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let info = mock_info("merchant", &coins(5, "uusd"));
    let msg = ExecuteMsg::AddCredits {
        contract_address: String::from("job1"),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!("job1", res.attributes[1].value);

    let info = mock_info("merchant", &coins(100000, "uusd"));
    let msg = ExecuteMsg::AddCredits {
        contract_address: String::from("job1"),
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = QueryMsg::GetJobCredits {
        contract_address: String::from("job1"),
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let value: NativeBalance = from_binary(&res).unwrap();

    assert!(value.has(&Coin::new(100005, "uusd")));
}

#[test]
fn work_receipt() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {};
    let info = mock_info("creator", &[]);
    // init
    let res = instantiate(deps.as_mut(), mock_env(), info.clone().clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());

    let msg = ExecuteMsg::UpdateAdmins {
        admins: vec!["merchant".to_owned()],
    };

    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Should not work on unregistered job
    let info = mock_info("job1", &vec![]);
    let msg = ExecuteMsg::WorkReceipt {
        worker_address: String::from("worker1"),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(ContractError::JobNotFound {}) => {}
        _ => panic!("Should not be able to work on non-existent jobs"),
    }

    let info = mock_info("merchant", &vec![]);
    let msg = ExecuteMsg::AddJob {
        contract_address: String::from("job1"),
        name: String::from("TestJob1"),
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // should work without fees
    let info = mock_info("job1", &vec![]);
    let msg = ExecuteMsg::WorkReceipt {
        worker_address: String::from("worker1"),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Ok(_) => {}
        _ => panic!("Should able to work when base_fee is empty"),
    }

    // set base_fee to 0.1 ust
    let info = mock_info("creator", &[]);
    let msg = ExecuteMsg::SetBaseFee {
        base_fee: vec![Coin::new(100000, "uusd")],
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // After setting fee, should not work without credits
    let info = mock_info("job1", &vec![]);
    let msg = ExecuteMsg::WorkReceipt {
        worker_address: String::from("worker1"),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(ContractError::JobInsufficientCredits {}) => {}
        _ => panic!("Should not be able to work without credits"),
    }

    // Adds some credits
    let info = mock_info("merchant", &coins(100000, "uusd"));
    let msg = ExecuteMsg::AddCredits {
        contract_address: String::from("job1"),
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    // Should send reward to worker
    let info = mock_info("job1", &vec![]);
    let msg = ExecuteMsg::WorkReceipt {
        worker_address: String::from("worker1"),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    for message in res.messages {
        assert_eq!("Bank(Send { to_address: \"worker1\", amount: [Coin { denom: \"uusd\", amount: Uint128(99009) }] })", format!("{:?}", message.msg));
    }

    assert_eq!("job1", res.attributes[1].value);
    assert_eq!("worker1", res.attributes[2].value);
    assert_eq!("100000uusd", res.attributes[3].value);

    let msg = QueryMsg::GetJobCredits {
        contract_address: String::from("job1"),
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let value: NativeBalance = from_binary(&res).unwrap();
    assert!(value.is_empty());

    // ran out of credits, test insufficient credits
    let info = mock_info("job1", &vec![]);
    let msg = ExecuteMsg::WorkReceipt {
        worker_address: String::from("worker1"),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(ContractError::JobInsufficientCredits {}) => {}
        _ => panic!("Should not be able to work without sufficient credits"),
    }
}
