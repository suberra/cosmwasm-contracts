#[cfg(test)]
mod tests {
    use crate::contract::{instantiate, query};
    use crate::msg::InstantiateMsg;
    use suberra_core::subwallet_factory::{QueryMsg, SubwalletFactoryConfig as Config};

    use cosmwasm_std::from_binary;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

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
}
