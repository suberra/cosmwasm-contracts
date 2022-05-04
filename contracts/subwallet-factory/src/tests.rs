#[cfg(test)]
mod tests {
    use crate::contract;
    use crate::contract::{instantiate, query};
    use crate::error::ContractError;
    use crate::msg::{ExecuteMsg, InstantiateMsg};
    use suberra_core::subwallet_factory::{QueryMsg, SubwalletFactoryConfig as Config};

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, Addr};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            subwallet_code_id: 17,
            anchor_market_contract: String::from("terra15dwd5mj8v59wpj0wvt233mf5efdff808c5tkal"),
            aterra_token_addr: String::from("terra1ajt556dpzvjwl0kl5tzku3fc3p3knkg9mkv8jl"),
        };

        // we can just call .unwrap() to assert this was a success
        let info = mock_info("deployer", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: Config = from_binary(&res).unwrap();
        assert_eq!(17, value.subwallet_code_id);
    }

    #[test]
    fn update_config() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            subwallet_code_id: 17,
            anchor_market_contract: String::from("terra15dwd5mj8v59wpj0wvt233mf5efdff808c5tkal"),
            aterra_token_addr: String::from("terra1ajt556dpzvjwl0kl5tzku3fc3p3knkg9mkv8jl"),
        };

        let info = mock_info("deployer", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let msg = ExecuteMsg::UpdateConfig {
            new_subwallet_code_id: Some(15u64),
            new_owner: Some("new_owner".to_string()),
            new_anchor_market_contract: None,
            new_aterra_token_addr: Some("new_token_addy".to_string()),
        };

        // unauthorised user attempts to change config. Should fail
        let unauth_info = mock_info("mallory", &coins(2, "token"));

        let res = contract::execute(deps.as_mut(), mock_env(), unauth_info, msg.clone());
        match res {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Contract should return an unauthorised error amount"),
        }

        let info = mock_info("deployer", &coins(2, "token"));
        let _res = contract::execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

        let msg = QueryMsg::Config {};
        let res = contract::query(deps.as_ref(), mock_env(), msg).unwrap();

        let config: Config = from_binary(&res).unwrap();
        let expected_config: Config = Config {
            owner: Addr::unchecked("new_owner"),
            aterra_token_addr: Addr::unchecked("new_token_addy"),
            anchor_market_contract: Addr::unchecked("terra15dwd5mj8v59wpj0wvt233mf5efdff808c5tkal"),
            subwallet_code_id: 15,
        };
        assert_eq!(config, expected_config);
    }
}
