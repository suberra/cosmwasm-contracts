use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use cosmwasm_std::{attr, Addr};

use subwallet_factory::msg::{ExecuteMsg, InstantiateMsg};

use suberra_core::subwallet_factory::{QueryMsg, SubwalletFactoryConfig as Config};
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

    let (factory_instance, subwallet_code_id) = instantiate_contracts(&mut app, owner.clone());

    assert_eq!(factory_instance, "contract #0");
    assert_eq!(subwallet_code_id, 1);

    let msg = QueryMsg::Config {};
    let res: Config = app
        .wrap()
        .query_wasm_smart(&factory_instance, &msg)
        .unwrap();

    assert_eq!(
        res,
        Config {
            subwallet_code_id,
            owner,
            anchor_market_contract: Addr::unchecked("terra15dwd5mj8v59wpj0wvt233mf5efdff808c5tkal"),
            aterra_token_addr: Addr::unchecked("terra1ajt556dpzvjwl0kl5tzku3fc3p3knkg9mkv8jl"),
        }
    );
}

#[test]
fn factory_creates_subwallet() {
    let mut app = mock_app();
    let owner = Addr::unchecked("owner");
    let alice = Addr::unchecked("alice");
    let bob = Addr::unchecked("bob");

    let (factory_instance, _subwallet_code_id) = instantiate_contracts(&mut app, owner.clone());

    let msg = ExecuteMsg::CreateAccount {};

    // Alice create the subwallet for the first time
    let res = app
        .execute_contract(
            Addr::unchecked(alice.clone()),
            factory_instance.clone(),
            &msg,
            &[],
        )
        .unwrap();

    assert_eq!(
        res.events[1].attributes[1],
        attr("action", "create_account")
    );

    // Alice cannot create the subwallet after the first time
    app.execute_contract(
        Addr::unchecked(alice.clone()),
        factory_instance.clone(),
        &msg,
        &[],
    )
    .unwrap_err();

    // Bob still can create the subwallets
    app.execute_contract(
        Addr::unchecked(bob.clone()),
        factory_instance.clone(),
        &msg,
        &[],
    )
    .unwrap();

    let msg = QueryMsg::GetSubwalletAddress {
        owner_address: alice.to_string(),
    };
    let res: String = app
        .wrap()
        .query_wasm_smart(&factory_instance, &msg)
        .unwrap();

    assert_eq!(res, "contract #1");

    let msg = QueryMsg::GetSubwalletAddress {
        owner_address: bob.to_string(),
    };
    let res: String = app
        .wrap()
        .query_wasm_smart(&factory_instance, &msg)
        .unwrap();

    assert_eq!(res, "contract #2");
}

/// instantiates subwallet_factory contract and stores subwallet_code returns the address of subwallet_factory and subwallet code id
fn instantiate_contracts(app: &mut TerraApp, owner: Addr) -> (Addr, u64) {
    let subwallet_code_id = store_subwallet_code(app);
    let factory_code_id = store_factory_code(app);

    let msg = InstantiateMsg {
        subwallet_code_id,
        anchor_market_contract: String::from("terra15dwd5mj8v59wpj0wvt233mf5efdff808c5tkal"),
        aterra_token_addr: String::from("terra1ajt556dpzvjwl0kl5tzku3fc3p3knkg9mkv8jl"),
    };

    let factory_instance = app
        .instantiate_contract(
            factory_code_id,
            Addr::unchecked(owner.clone()),
            &msg,
            &[],
            "subwallet_factory",
            None,
        )
        .unwrap();

    (factory_instance, subwallet_code_id)
}

fn store_factory_code(app: &mut TerraApp) -> u64 {
    let factory_contract = Box::new(
        ContractWrapper::new_with_empty(
            subwallet_factory::contract::execute,
            subwallet_factory::contract::instantiate,
            subwallet_factory::contract::query,
        )
        .with_reply_empty(subwallet_factory::contract::reply),
    );

    app.store_code(factory_contract)
}

fn store_subwallet_code(app: &mut TerraApp) -> u64 {
    let subwallet_contract = Box::new(ContractWrapper::new_with_empty(
        subwallet::contract::execute,
        subwallet::contract::instantiate,
        subwallet::contract::query,
    ));

    app.store_code(subwallet_contract)
}
