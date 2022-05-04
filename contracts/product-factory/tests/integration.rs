use cosmwasm_bignumber::Uint256;
use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use cosmwasm_std::{attr, Addr};
use suberra_core::msg::JobsRegistryInstantiateMsg;
use suberra_core::product_factory::{
    ConfigResponse as ProductFactoryConfigResponse, CreateProductExecuteMsg, ExecuteMsg,
    InstantiateMsg, ProductsResponse, QueryMsg,
};

use terra_multi_test::{AppBuilder, BankKeeper, ContractWrapper, Executor, TerraApp, TerraMock};

fn mock_app() -> TerraApp {
    let env = mock_env();
    let api = MockApi::default();
    let bank = BankKeeper::new();
    let storage = MockStorage::new();
    let custom = TerraMock::luna_ust_case();

    AppBuilder::new()
        .with_api(api)
        .with_block(env.block)
        .with_bank(bank)
        .with_storage(storage)
        .with_custom(custom)
        .build()
}

#[test]
fn proper_initialization_factory() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");

    let product_code_id = store_product_code(&mut app);
    let factory_code_id = store_factory_code(&mut app);

    let msg = InstantiateMsg {
        product_code_id: product_code_id,
        protocol_fee_bps: 100,
        min_amount_per_interval: Uint256::from(100u64),
        min_protocol_fee: Uint256::zero(),
        fee_address: "owner".to_string(),
        job_registry_address: "jobs".to_string(),
    };

    let factory_instance = app
        .instantiate_contract(
            factory_code_id,
            Addr::unchecked(owner.clone()),
            &msg,
            &[],
            "factory",
            None,
        )
        .unwrap();

    assert_eq!(factory_instance, "contract #0");
}

#[test]
fn factory_creates_product() {
    let mut app = mock_app();
    let owner = Addr::unchecked("owner");

    let (_jobs_registry_instance, factory_instance, _product_code_id) =
        instantiate_contracts(&mut app, owner.clone());

    // creates a product
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

    // create the product for the first time
    let res = app
        .execute_contract(
            Addr::unchecked(owner.clone()),
            factory_instance.clone(),
            &msg,
            &[],
        )
        .unwrap();

    assert_eq!(
        res.events[1].attributes[1],
        attr("action", "create_product")
    );

    let product_instance = &res.events[3].attributes[0].value;

    assert_eq!(res.events[5].attributes[2], attr("product_id", "1"));

    // checks to make sure contract is correctly added into the job registry contract
    assert_eq!(
        res.events[7].attributes[1..4],
        vec![
            attr("method", "try_add_job"),
            attr("contract", product_instance),
            attr("job_id", "1")
        ]
    );

    // create the second product
    let res = app
        .execute_contract(
            Addr::unchecked(owner.clone()),
            factory_instance.clone(),
            &msg,
            &[],
        )
        .unwrap();

    assert_eq!(
        res.events[1].attributes[1],
        attr("action", "create_product")
    );
    assert_eq!(res.events[5].attributes[2], attr("product_id", "2")); // product_id should increment by one
}

#[test]
fn test_update_config() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");

    let product_code_id = store_product_code(&mut app);
    let factory_code_id = store_factory_code(&mut app);

    let msg = InstantiateMsg {
        product_code_id: product_code_id,
        protocol_fee_bps: 100,
        min_amount_per_interval: Uint256::from(100u64),
        min_protocol_fee: Uint256::zero(),
        fee_address: "owner".to_string(),
        job_registry_address: "jobs".to_string(),
    };

    let factory_instance = app
        .instantiate_contract(
            factory_code_id,
            Addr::unchecked(owner.clone()),
            &msg,
            &[],
            "factory",
            None,
        )
        .unwrap();
    assert_eq!(factory_instance, "contract #0");

    let msg = ExecuteMsg::UpdateConfig {
        new_owner: Some("new_owner".to_string()),
        new_fee_address: Some("fee2".to_string()),
        new_job_registry_address: Some("jobs2".to_string()),
        new_product_code_id: Some(5u64),
        new_protocol_fee_bps: Some(500u64),
        new_min_protocol_fee: Some(Uint256::zero()),
        new_min_amount_per_interval: None,
    };
    let res = app
        .execute_contract(
            Addr::unchecked(owner.clone()),
            factory_instance.clone(),
            &msg,
            &[],
        )
        .unwrap();

    assert_eq!(
        res.events[1].attributes[1..],
        vec![
            attr("method", "update_config"),
            attr("new_owner", "new_owner"),
            attr("new_protocol_fee_bps", "500"),
            attr("new_min_protocol_fee", "0"),
            attr("new_product_code_id", "5"),
            attr("new_fee_address", "fee2"),
            attr("new_job_registry_address", "jobs2")
        ]
    );

    // query config

    let msg = QueryMsg::Config {};
    let res: ProductFactoryConfigResponse = app
        .wrap()
        .query_wasm_smart(&factory_instance, &msg)
        .unwrap();

    let expected: ProductFactoryConfigResponse = ProductFactoryConfigResponse {
        owner: "new_owner".to_string(),
        product_code_id: 5,
        protocol_fee_bps: 500,
        min_protocol_fee: Uint256::zero(),
        fee_address: "fee2".to_string(),
        job_registry_address: "jobs2".to_string(),
    };

    assert_eq!(res, expected);
}

#[test]
fn test_query_products() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");

    let (_jobs_registry_instance, factory_instance, _product_code_id) =
        instantiate_contracts(&mut app, owner.clone());

    assert_eq!(factory_instance, Addr::unchecked("contract #1"));

    // creates a product
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

    // create the product for the first time
    app.execute_contract(
        Addr::unchecked(owner.clone()),
        factory_instance.clone(),
        &msg,
        &[],
    )
    .unwrap();

    // create the second product
    app.execute_contract(
        Addr::unchecked(owner.clone()),
        factory_instance.clone(),
        &msg,
        &[],
    )
    .unwrap();

    // alice create the 3rd product
    app.execute_contract(
        Addr::unchecked("alice".clone()),
        factory_instance.clone(),
        &msg,
        &[],
    )
    .unwrap();

    // Query products by owner
    let msg = QueryMsg::ProductsByOwner {
        owner: "owner".to_string(),
        start_after: None,
        limit: None,
    };
    let res: ProductsResponse = app
        .wrap()
        .query_wasm_smart(&factory_instance, &msg)
        .unwrap();

    assert_eq!(
        res.products,
        vec![
            Addr::unchecked("contract #2"),
            Addr::unchecked("contract #3")
        ]
    );
    assert_eq!(res.last_key, Some(2));

    // Query products by owner
    let msg = QueryMsg::ProductsByOwner {
        owner: "alice".to_string(),
        start_after: None,
        limit: None,
    };
    let res: ProductsResponse = app
        .wrap()
        .query_wasm_smart(&factory_instance, &msg)
        .unwrap();

    assert_eq!(res.products, vec![Addr::unchecked("contract #4"),]);
    assert_eq!(res.last_key, Some(3));

    // Query all pagingated products
    let msg = QueryMsg::ProductsByOwner {
        owner: "owner".to_string(),
        start_after: Some(1),
        limit: None,
    };
    let res: ProductsResponse = app
        .wrap()
        .query_wasm_smart(&factory_instance, &msg)
        .unwrap();

    assert_eq!(res.products, vec![Addr::unchecked("contract #3"),]);

    assert_eq!(res.last_key, Some(2));
}
/// instantiates jobs contract and factory contract and returns the address of jobs contract, factory_contract, product_id of the code.
fn instantiate_contracts(app: &mut TerraApp, owner: Addr) -> (Addr, Addr, u64) {
    let product_code_id = store_product_code(app);
    let factory_code_id = store_factory_code(app);
    let jobs_code_id = store_jobs_registry_code(app);

    // instantiates the jobs contracts
    let jobs_instance = app
        .instantiate_contract(
            jobs_code_id,
            owner.clone(),
            &JobsRegistryInstantiateMsg {},
            &[],
            "jobs_registry",
            None,
        )
        .unwrap();

    let msg = InstantiateMsg {
        product_code_id: product_code_id,
        protocol_fee_bps: 100,
        min_amount_per_interval: Uint256::from(100u64),
        min_protocol_fee: Uint256::zero(),
        fee_address: owner.to_string(),
        job_registry_address: jobs_instance.to_string(),
    };

    let factory_instance = app
        .instantiate_contract(
            factory_code_id,
            Addr::unchecked(owner.clone()),
            &msg,
            &[],
            "factory",
            None,
        )
        .unwrap();

    (jobs_instance, factory_instance, product_code_id)
}

fn store_factory_code(app: &mut TerraApp) -> u64 {
    let factory_contract = Box::new(
        ContractWrapper::new_with_empty(
            product_factory::contract::execute,
            product_factory::contract::instantiate,
            product_factory::contract::query,
        )
        .with_reply_empty(product_factory::contract::reply),
    );

    app.store_code(factory_contract)
}

fn store_product_code(app: &mut TerraApp) -> u64 {
    let product_contract = Box::new(ContractWrapper::new_with_empty(
        sub1_fixed_recurring_subscriptions::contract::execute,
        sub1_fixed_recurring_subscriptions::contract::instantiate,
        sub1_fixed_recurring_subscriptions::contract::query,
    ));

    app.store_code(product_contract)
}

fn store_jobs_registry_code(app: &mut TerraApp) -> u64 {
    let jobs_contract = Box::new(ContractWrapper::new_with_empty(
        jobs_registry::contract::execute,
        jobs_registry::contract::instantiate,
        jobs_registry::contract::query,
    ));

    app.store_code(jobs_contract)
}
