use crate::error::ContractError;
use crate::msg::WorkPayload;
use crate::msg::{
    ConfigResponse, ExecuteMsg, JobsRegistryExecuteMsg, QueryMsg, SubscriptionInfoResponse,
    SubscriptionsResponse,
};
use crate::querier::get_job_registry;
use crate::state::{
    create_subscription, read_config, store_config, Config, SubscriptionInfo, SUBSCRIPTIONS,
};
use cosmwasm_bignumber::{Decimal256, Uint256};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, Api, Binary, CosmosMsg, Deps, DepsMut, Empty, Env,
    MessageInfo, Order, Response, StdResult, Storage, Timestamp, Uint128, WasmMsg,
};
use cw_storage_plus::Bound;

use admin_core::{
    contract::{instantiate as whitelist_instantiate, query_admin_list, query_owner},
    error::ContractError as AdminCoreContractError,
    msg::InstantiateMsg as CW1InitMsg,
    state::ADMIN_CONFIG,
};

use suberra_core::msg::{MigrateMsg, ProductInstantiateMsg, SubWalletExecuteMsg};
use suberra_core::querier::query_product_factory_config;
use suberra_core::subscriptions::Discount;
use suberra_core::util::optional_addr_validate;

const DEFAULT_LIMIT: u32 = 10;
const MAX_FEE_DECIMAL: u64 = 10_000u64; // constant for 100%
const MAX_LIMIT: u32 = 30;
const DEFAULT_GRACE_PERIOD: u64 = 86400; // 24 hours in seconds

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ProductInstantiateMsg,
) -> Result<Response, AdminCoreContractError> {
    let owner_address = deps.api.addr_validate(&msg.owner)?;

    let _result = whitelist_instantiate(
        deps.branch(),
        env,
        info,
        CW1InitMsg {
            owner: owner_address.to_string(),
            admins: msg.admins,
            mutable: msg.mutable,
        },
    )?;

    // unit_interval needs to be converted from hours to Timestamp
    let unit_interval = Timestamp::from_seconds(msg.unit_interval_hour * 60 * 60);

    let additional_grace_period = match msg.additional_grace_period_hour {
        Some(v) => v * 60 * 60,
        None => 0,
    };

    store_config(
        deps.storage,
        &Config {
            owner_address: owner_address.clone(),
            receiver_address: deps.api.addr_validate(&msg.receiver_address)?,
            additional_grace_period,
            unit_interval,
            unit_amount: msg.unit_amount,
            initial_amount: msg.initial_amount,
            uri: msg.uri,
            paused: false,
            factory_address: deps.api.addr_validate(&msg.factory_address)?,
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        // required attribute for factory to register owner
        .add_attribute("owner", owner_address.to_string()))
}

/// Available the execute messages of the contract.
/// ## Params
/// * **deps** is the object of type [`Deps`].
///
/// * **env** is the object of type [`Env`].
///
/// * **info** is the object of type [`MessageInfo`].
///
/// * **msg** is the object of type [`ExecuteMsg`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<Empty>, ContractError> {
    let api = deps.api;
    match msg {
        ExecuteMsg::UpdateConfig {
            receiver_address,
            additional_grace_period_hour,
            initial_amount,
            uri,
        } => {
            let api = deps.api;
            update_config(
                deps,
                env,
                info,
                optional_addr_validate(api, receiver_address)?,
                initial_amount,
                additional_grace_period_hour,
                uri,
            )
        }
        ExecuteMsg::UpdateAdmins { admins } => execute_update_admins(deps, env, info, admins),
        ExecuteMsg::Subscribe {} => execute_subscribe(deps, info, env),
        ExecuteMsg::Cancel {} => execute_cancel(deps, info, env),
        ExecuteMsg::Pause {} => execute_pause(deps, info, env, true),
        ExecuteMsg::Unpause {} => execute_pause(deps, info, env, false),
        ExecuteMsg::ModifySubscriber {
            new_created_at,
            new_last_charged,
            new_interval_end_at,
            subscriber,
        } => execute_modify_subscriber(
            deps,
            info,
            env,
            new_created_at,
            new_last_charged,
            new_interval_end_at,
            api.addr_validate(&subscriber)?,
        ),
        ExecuteMsg::SetDiscount {
            discount,
            subscriber,
        } => execute_set_discount(deps, info, env, discount, api.addr_validate(&subscriber)?),
        ExecuteMsg::RemoveSubscriber { subscriber } => {
            execute_remove_subscriber(deps, info, env, api.addr_validate(&subscriber)?)
        }
        ExecuteMsg::Charge { payer_address } => {
            execute_charge(deps, env, api.addr_validate(&payer_address)?, None)
        }
        ExecuteMsg::Work { payload } => {
            let work_payload: WorkPayload = from_binary(&payload).unwrap();
            execute_work(
                deps,
                info,
                env,
                api.addr_validate(&work_payload.payer_address)?,
            )
        }
    }
}

#[allow(dead_code)]
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}

/// Updates the generate settings.
///
/// ## Executor
/// Only owner or admin can execute this function
#[allow(clippy::too_many_arguments)]
pub fn update_config(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    receiver_address: Option<Addr>,
    initial_amount: Option<Uint256>,
    additional_grace_period_hour: Option<u64>,
    uri: Option<String>,
) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;
    let mut attributes = Vec::from([
        attr("method", "update_config"),
        attr("module_contract_address", env.contract.address.to_string()),
    ]);

    // Only owner or admin can call this function
    let cfg = ADMIN_CONFIG.load(deps.storage)?;
    if !cfg.is_admin(info.sender.as_ref()) && info.sender != config.owner_address {
        return Err(ContractError::Unauthorized {});
    }

    // only `receiver_address`, `additional_grace_period_hour` and `uri` can be updated. Only update if a value is given

    if let Some(receiver_address) = receiver_address {
        config.receiver_address = deps.api.addr_validate(receiver_address.as_str())?;
    }

    if let Some(initial_amount) = initial_amount {
        config.initial_amount = initial_amount;
        attributes.push(attr("new_initial_amount", initial_amount));
    }

    if let Some(additional_grace_period_hour) = additional_grace_period_hour {
        config.additional_grace_period = additional_grace_period_hour * 60 * 60
    }

    if let Some(uri) = uri {
        config.uri = uri
    }

    store_config(deps.storage, &config)?;

    Ok(Response::new().add_attributes(attributes))
}

/// Updates the admins for the subscription object.
///
/// ## Executor
/// Can only be peformed by the owner

pub fn execute_update_admins(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    admins: Vec<String>,
) -> Result<Response, ContractError> {
    let mut cfg = ADMIN_CONFIG.load(deps.storage)?;
    let config: Config = read_config(deps.storage)?;

    if info.sender != config.owner_address {
        Err(ContractError::Unauthorized {})
    } else {
        cfg.admins = map_validate(deps.api, &admins)?;
        ADMIN_CONFIG.save(deps.storage, &cfg)?;

        let res = Response::new().add_attribute("action", "update_admins");
        Ok(res)
    }
}

pub fn map_validate(api: &dyn Api, admins: &[String]) -> StdResult<Vec<Addr>> {
    admins.iter().map(|addr| api.addr_validate(addr)).collect()
}

// calculates the protocol fee that will be payable to the suberra protocol. `protocol_fee_bps` is queried from the factory.
// returns None is there is no fee that is payable. Otherwise returns the amount payable to protocola
pub fn calculate_protocol_fee(
    protocol_fee_bps: u64,
    min_protocol_fee: Uint256,
    amount: Uint256,
) -> Option<Uint256> {
    if protocol_fee_bps == 0 {
        return None;
    }

    let protocol_fee_rate = Decimal256::from_ratio(
        Uint256::from(protocol_fee_bps),
        Uint256::from(MAX_FEE_DECIMAL),
    );

    // get the protocol_fee. If the protocol_fee is less than the minimum protocol_fee, the minimum protocol_fee should be used
    let protocol_fee = std::cmp::max(protocol_fee_rate * amount, min_protocol_fee);

    Some(protocol_fee)
}

/// Creates a subscription object whenever the user subscribes to the product.
/// If the `initial_amount` is set to a non-zero value, this function should process the payment from subscriber to merchant (and protocol, if applicable)
///
/// * **deps** is the object of [`DepsMut`]
///
///  * **_info** is the object of type [`MessageInfo`]
///
///  * **_env** is the object of type [`Env`]
///
pub fn execute_subscribe(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let mut is_undo = false; // flag on whether this is an undo cancellation request

    if config.paused {
        return Err(ContractError::Paused {});
    }

    let mut msgs = Vec::new();
    let mut attributes = Vec::new();

    let subscriber = info.sender.clone();
    let new_subscription = SubscriptionInfo {
        owner: deps.api.addr_validate(subscriber.as_str())?,
        created_at: env.block.time,
        interval_end_at: env.block.time.plus_seconds(config.unit_interval.seconds()),
        last_charged: Timestamp::from_seconds(0u64),
        is_cancelled: false,
        discount_per_interval: None,
    };

    let get_subscription = SUBSCRIPTIONS.may_load(deps.storage, &subscriber)?;

    let mut subscription = match get_subscription {
        Some(mut current_subscription) => {
            // If the user have an existing subscription, check if it is active.
            let subscription_active =
                is_subscription_active(deps.storage, env.clone(), current_subscription.clone());
            if subscription_active && !current_subscription.is_cancelled {
                // return error if the existing subscription is not cancelled
                return Err(ContractError::ExistingSubscriptionFound {});
            } else {
                match subscription_active && current_subscription.is_cancelled {
                    true => {
                        // subscriber had a subscription that he/she has previously cancelled, but has not reached its expiry
                        // In circumstances like this, it is treated as a undo_cancellation
                        // no change to the previous subscription period
                        is_undo = true;
                        attributes.push(attr("additional_info", "undo_cancellation"));

                        current_subscription.is_cancelled = false;
                        current_subscription
                    }
                    false => new_subscription,
                }
            }
        }
        _ => new_subscription,
    };

    // get fee info from factory
    let fee = query_product_factory_config(&deps.querier, config.factory_address.clone())?;

    // protocol fee cannot be more than 100% as it is invalid
    if fee.protocol_fee_bps > MAX_FEE_DECIMAL {
        return Err(ContractError::InvalidFee {});
    }

    // handling scenario where an initial_amount is required to kickstart the subscription
    if !config.initial_amount.is_zero() && !is_undo {
        subscription.last_charged = env.block.time;

        let protocol_fee = calculate_protocol_fee(
            fee.protocol_fee_bps,
            fee.min_protocol_fee,
            config.initial_amount,
        );

        // Amount to pay merchant
        let mut merchant_amount = config.initial_amount;

        // computes the protocol fees payable if protocol_fee is non-zero
        if let Some(protocol_fee) = protocol_fee {
            merchant_amount = merchant_amount - protocol_fee;

            // push a message for the fees payment
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: subscriber.clone().into_string(),
                funds: vec![],
                msg: to_binary(&SubWalletExecuteMsg::TransferAToken {
                    recipient: fee.fee_address,
                    amount: Uint128::from(protocol_fee),
                })?,
            }));
        }

        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: subscriber.clone().into_string(),
            funds: vec![],
            msg: to_binary(&SubWalletExecuteMsg::TransferAToken {
                recipient: config.receiver_address.to_string(),
                amount: Uint128::from(merchant_amount),
            })?,
        }));

        attributes.push(attr("initial_amount", config.initial_amount));
    }

    // make state changes for the subscription object
    create_subscription(
        deps.storage,
        deps.api.addr_validate(info.sender.as_str())?,
        subscription,
    )?;

    attributes.push(attr("method", "execute_subscribe"));
    attributes.push(attr("result", "subscribe_success"));
    attributes.push(attr("subscriber", subscriber.into_string()));
    attributes.push(attr(
        "module_contract_address",
        env.contract.address.to_string(),
    ));

    Ok(Response::new()
        .add_messages(msgs)
        .add_attributes(attributes))
}

/// Allows the user to cancel its own subscription. Once cancelled, workers will not be able to call charge again to initiate a new subscription.
/// The user's existing subscription will also stay valid until the end of the period
pub fn execute_cancel(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;

    if config.paused {
        return Err(ContractError::Paused {});
    }

    let subscriber = info.sender;

    // get receiver address
    let mut subscription = match SUBSCRIPTIONS.may_load(deps.storage, &subscriber.clone())? {
        Some(v) => v,
        None => return Err(ContractError::SubscriptionNotFound {}),
    };

    if subscription.is_cancelled {
        return Err(ContractError::SubscriptionCancelled {});
    }

    subscription.is_cancelled = true;

    SUBSCRIPTIONS.save(deps.storage, &subscriber, &subscription)?;

    Ok(Response::new().add_attributes(vec![
        attr("method", "execute_cancel"),
        attr("module_contract_address", env.contract.address.to_string()),
    ]))
}

/// Remove subscriber from the contract. Once removed, subscriber loses access to the services immediately without refunds for unused period.
///
/// ## Executor
/// Only owner or admin can execute this function
///
pub fn execute_remove_subscriber(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    subscriber: Addr,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;

    if config.paused {
        return Err(ContractError::Paused {});
    }

    // Only owner or admin can call this function
    let cfg = ADMIN_CONFIG.load(deps.storage)?;
    if !cfg.is_admin(info.sender.as_ref()) && info.sender != config.owner_address {
        return Err(ContractError::Unauthorized {});
    }

    match SUBSCRIPTIONS.may_load(deps.storage, &subscriber.clone())? {
        Some(_) => {
            // remove_subscriber removes the susbcriber from the Map, revoking its access to the platform immediately without refunds
            SUBSCRIPTIONS.remove(deps.storage, &subscriber);
        }
        None => return Err(ContractError::SubscriptionNotFound {}),
    };

    Ok(Response::new().add_attributes(vec![
        attr("method", "execute_remove_subscriber"),
        attr("module_contract_address", env.contract.address.to_string()),
    ]))
}

/// Set or unset the discount for subscribers.
/// Only this method can modify the Discount status. ModifySubscriber will not be able to modify the Discount section.
pub fn execute_set_discount(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    discount: Option<Discount>,
    subscriber: Addr,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let cfg = ADMIN_CONFIG.load(deps.storage)?;

    let attributes = Vec::from([
        attr("method", "set_discount"),
        attr("module_contract_address", env.contract.address.to_string()),
    ]);

    if config.paused {
        return Err(ContractError::Paused {});
    }

    // Only owner or admin can call this function
    if !cfg.is_admin(info.sender.as_ref()) && info.sender != config.owner_address {
        return Err(ContractError::Unauthorized {});
    }

    // get subscriber
    let mut subscription = match SUBSCRIPTIONS.may_load(deps.storage, &subscriber.clone())? {
        Some(v) => v,
        None => return Err(ContractError::SubscriptionNotFound {}),
    };

    // validate discount
    if !is_valid_discount(discount.clone(), config.unit_amount) {
        return Err(ContractError::InvalidDiscount {});
    }

    // the admin or owner should be able to set a subscriber's discount to None.
    subscription.discount_per_interval = discount;

    SUBSCRIPTIONS.save(deps.storage, &subscriber, &subscription)?;
    Ok(Response::new().add_attributes(attributes))
}

// checks if a discount is valid. If valid, returns true. Otherwise return false.
fn is_valid_discount(discount: Option<Discount>, subscription_amount: Uint256) -> bool {
    if let Some(discount) = discount {
        // checks if the amount if valid
        if discount.amount > subscription_amount {
            return false;
        }
    }
    // returns true. Setting discount to be None should be valid
    true
}

/// Modify settings for a given subscriber. Owner should be able to change the `last_charged, `created_at` and `interval_end_at` timestamp.
/// To modify the Discount status for a user, use the `ExecuteMsg::SetDiscount` message instead
///
/// ## Executor
/// Only owner or admin can execute this function
///
#[allow(clippy::too_many_arguments)]
pub fn execute_modify_subscriber(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    new_created_at: Option<u64>,
    new_last_charged: Option<u64>,
    new_interval_end_at: Option<u64>,
    subscriber: Addr,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let cfg = ADMIN_CONFIG.load(deps.storage)?;

    let mut attributes = Vec::from([
        attr("method", "modify_subscriber"),
        attr("module_contract_address", env.contract.address.to_string()),
    ]);

    if config.paused {
        return Err(ContractError::Paused {});
    }

    // Only owner or admin can call this function
    if !cfg.is_admin(info.sender.as_ref()) && info.sender != config.owner_address {
        return Err(ContractError::Unauthorized {});
    }

    // get subscriber
    let mut subscription = match SUBSCRIPTIONS.may_load(deps.storage, &subscriber.clone())? {
        Some(v) => v,
        None => return Err(ContractError::SubscriptionNotFound {}),
    };

    // updates the subscription value if applicable
    if let Some(created_at) = new_created_at {
        // created_at must be in the past
        if created_at > env.block.time.seconds() {
            return Err(ContractError::InvalidParam {});
        }

        subscription.created_at = Timestamp::from_seconds(created_at);
        attributes.push(attr("new_created_at", created_at.to_string()));
    }

    if let Some(interval_end_at) = new_interval_end_at {
        // interval_end_at must be some time in the future
        if interval_end_at <= env.block.time.seconds() {
            return Err(ContractError::InvalidParam {});
        }

        subscription.interval_end_at = Timestamp::from_seconds(interval_end_at);
        attributes.push(attr("new_interval_end_at", interval_end_at.to_string()));
    }

    if let Some(last_charged) = new_last_charged {
        // last_charged be some time in the past or present, but not in the future.
        if last_charged > env.block.time.seconds() {
            return Err(ContractError::InvalidParam {});
        }

        subscription.last_charged = Timestamp::from_seconds(last_charged);
        attributes.push(attr("new_last_charged", last_charged.to_string()));
    }

    SUBSCRIPTIONS.save(deps.storage, &subscriber, &subscription)?;

    Ok(Response::new().add_attributes(attributes))
}

/// `execute_work` is called by a Worker who is performing upkeep for the contract
pub fn execute_work(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    payer_address: Addr,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;

    if config.paused {
        return Err(ContractError::Paused {});
    }

    let worker = info.sender.to_string();

    // get the job registry contract address
    let job_registry_address = get_job_registry(deps.as_ref())?;

    // execute_work differs from execute_charge due to the optional_message that must be sent back to the `job_regisexecute contract`
    execute_charge(
        deps,
        env,
        payer_address,
        Some(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: job_registry_address.to_string(),
            funds: vec![],
            msg: to_binary(&JobsRegistryExecuteMsg::WorkReceipt {
                worker_address: worker,
            })?,
        })),
    )
}

/// Charges the payer (subscriber), where the contract will attempt to transfer tokens to the receiving address.
/// If successful, the [`SubscriptionInfo`] object will be changed accordingly.
/// ## Params
/// * **deps** is the object of type [`DepsMut`].
///
/// * **_env** is the object of type [`Env`].
///
/// * **subscriber** is the address of the payer of type [`Addr`]
///
/// * **additional_message** is any [`Option`] type additional message that the caller could attach to the message. Required for job registry to work properly.
///
pub fn execute_charge(
    deps: DepsMut,
    env: Env,
    subscriber: Addr,
    additional_message: Option<CosmosMsg>,
) -> Result<Response, ContractError> {
    // checks if the contract is paused. If paused, do not proceed to charge existing subscribers
    let config = read_config(deps.storage)?;

    if config.paused {
        return Err(ContractError::Paused {});
    }

    // get receiver address
    let subscription = match SUBSCRIPTIONS.may_load(deps.storage, &subscriber.clone())? {
        Some(v) => v,
        None => return Err(ContractError::SubscriptionNotFound {}),
    };

    if subscription.is_cancelled {
        return Err(ContractError::SubscriptionCancelled {});
    }

    // charge should not be possible if the subscription is no longer active (i.e. past due and falls outside of grace period)
    // To resume subscription, subscriber should resubscribe again
    if !is_subscription_active(deps.storage, env.clone(), subscription.clone()) {
        // throw error CannotCharge if the subscription is no longer active. Workers should not be able to charge.
        return Err(ContractError::CannotCharge {});
    }

    // gets the chargeable amount after discount if any
    let chargeable_amount: AmountTransferable =
        get_chargeable_amount(deps.as_ref(), &env, &subscription)?;

    if chargeable_amount.amount.is_zero() {
        return Err(ContractError::NoCharge {});
    }

    let mut updated_subscription = subscription.clone();
    updated_subscription.last_charged = env.block.time;
    updated_subscription.interval_end_at = subscription
        .interval_end_at
        .plus_seconds(config.unit_interval.seconds() * chargeable_amount.number_of_intervals);

    // update the subscription object
    SUBSCRIPTIONS.save(deps.storage, &subscriber, &updated_subscription)?;

    // get fee info from factory
    let fee = query_product_factory_config(&deps.querier, config.factory_address.clone())?;

    // fee cannot be more than 100% and fee must not be more than chargeable_amount
    if fee.protocol_fee_bps > MAX_FEE_DECIMAL || fee.min_protocol_fee > chargeable_amount.amount {
        return Err(ContractError::InvalidFee {});
    }

    let mut msgs: Vec<CosmosMsg> = Vec::new();

    let protocol_fee = calculate_protocol_fee(
        fee.protocol_fee_bps,
        fee.min_protocol_fee,
        chargeable_amount.amount,
    );

    let mut merchant_amount = chargeable_amount.amount;

    if let Some(protocol_fee) = protocol_fee {
        // updates the amt with fees
        merchant_amount = merchant_amount - protocol_fee;

        // append message for fee collection
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: subscriber.clone().into_string(),
            funds: vec![],
            msg: to_binary(&SubWalletExecuteMsg::TransferAToken {
                recipient: fee.fee_address,
                amount: Uint128::from(protocol_fee),
            })?,
        }));
    }

    // append message to send fees to the merchant
    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: subscriber.clone().into_string(),
        funds: vec![],
        msg: to_binary(&SubWalletExecuteMsg::TransferAToken {
            recipient: config.receiver_address.to_string(),
            amount: Uint128::from(merchant_amount),
        })?,
    }));

    // pushes the additional messages if there is any.
    // Required if the worker wants to push some messages relating to the claim fees
    if let Some(msg) = additional_message {
        msgs.push(msg);
    }

    Ok(Response::new().add_messages(msgs).add_attributes(vec![
        attr("method", "execute_charge"),
        attr("module_contract_address", env.contract.address.to_string()),
        attr("subscriber", subscriber.into_string()),
        attr("amount", chargeable_amount.amount.to_string()),
        attr("periods", chargeable_amount.number_of_intervals.to_string()),
    ]))
}

/// Wrapper function to call the [`compute_amount_chargeable`]
pub fn get_chargeable_amount(
    deps: Deps,
    env: &Env,
    subscription: &SubscriptionInfo,
) -> Result<AmountTransferable, ContractError> {
    let config: Config = read_config(deps.storage)?;
    Ok(compute_amount_chargeable(
        &config,
        subscription,
        env.block.time,
    ))
}

/// Data Structure that will be returned to the [`execute_charge`] function. Stores required information to make adjustments to [`SubscriptionInfo`]
/// amount: Chargeable amount after factoring discounts and max cap
/// number_of_intervals: Number of periods that the amount should be charged for
pub struct AmountTransferable {
    pub amount: Uint256,
    pub number_of_intervals: u64,
}

/// Computes the amount chargeable for the subscription and the period adjustment to be made for the [`SubscriptionInfo`] object.
/// Factors in the discount if [`Discount`] object is set for the particular subscriber.
/// Returns an object of type [`AmountTransferable`]. If no charge can be made, the object with have zero value for the amount and number_of_intervals
pub fn compute_amount_chargeable(
    config: &Config,
    subscription: &SubscriptionInfo,
    block_time: Timestamp,
) -> AmountTransferable {
    // if subscription is not due, return zero
    if block_time < subscription.interval_end_at {
        return AmountTransferable {
            amount: Uint256::zero(),
            number_of_intervals: 0u64,
        };
    }

    // calculates elapsed time from the last interval_end_at timestamp
    let elapsed_time = block_time.minus_seconds(subscription.interval_end_at.seconds());

    // calculate the number of interval (rounded down)
    let interval = elapsed_time
        .plus_seconds(config.unit_interval.seconds())
        .seconds()
        / config.unit_interval.seconds();

    // checks for amount after discount
    // eligible_discount is the discount that will be applied to the actual final_amount after factoring in the expiry (if any)
    let eligible_discount = match &subscription.discount_per_interval {
        Some(discount) => {
            if discount.amount > config.unit_amount {
                // discount more than unit_amount, returns unit_amount
                config.unit_amount
            } else {
                discount.amount
            }
        }
        None => Uint256::zero(),
    };

    let interval_amount_after_discount = config.unit_amount - eligible_discount;

    let chargeable_amount = Uint256::from(interval) * interval_amount_after_discount;

    AmountTransferable {
        amount: chargeable_amount,
        number_of_intervals: interval,
    }
}

/// Pause or unpause the contract. Takes in a value `execute_pause` of [`bool`] type.
/// Once paused, no other operations on the contractwill be allowed. Users still have their [`SubscriptionInfo`] stored on the contract.
/// Be careful of the effects of pausing and unpausing a contract as it might result in disruptive user experience. User's last_charged and membership validity might be affected.
/// Returns an [`ContractError`] on failure or returns the [`Response`] with the specified attributes
/// if the operation was successful.
///
/// ## Params
/// * **deps** is the object of type [`DepsMut`].
///
/// * **info** is the object of type [`MessageInfo`].
///
///  * **_env** is the object of type [`Env`]
///
/// * **execute_pause** is the object of type [`bool`]. `true` if admin is attempting to pause it,`false` if admin is attempting to unpause it (resume operations)
pub fn execute_pause(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    execute_pause: bool,
) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;
    let cfg = ADMIN_CONFIG.load(deps.storage)?;

    // Only owner can call this function
    if !cfg.is_admin(info.sender.as_ref()) && info.sender != config.owner_address {
        return Err(ContractError::Unauthorized {});
    }

    if execute_pause {
        if config.paused {
            return Err(ContractError::Paused {});
        }
    } else {
        // user executing to unpause
        if !config.paused {
            return Err(ContractError::Unpaused {});
        }
    }

    config.paused = !config.paused;
    store_config(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![
        attr("method", "execute_flip_pause"),
        attr("module_contract_address", env.contract.address.to_string()),
        attr("is_paused", config.paused.to_string()),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::AdminConfig {} => to_binary(&query_admin_list(deps)?),
        QueryMsg::Owner {} => to_binary(&query_owner(deps)?),
        QueryMsg::Subscription { subscriber } => {
            to_binary(&query_subscription(deps, env, subscriber)?)
        }
        QueryMsg::Subscriptions { start_after, limit } => {
            to_binary(&query_subscriptions(deps, start_after, limit)?)
        }
        QueryMsg::CanWork { payload } => {
            let work_payload: WorkPayload = from_binary(&payload).unwrap();
            to_binary(&query_can_work(
                deps,
                env,
                deps.api.addr_validate(&work_payload.payer_address)?,
            )?)
        }
    }
}

/// query_can_work is called by the Worker nodes - they will query periodically and only perform work when there is a valid work to be done
fn query_can_work(deps: Deps, env: Env, subscriber: Addr) -> StdResult<bool> {
    let subscription = match SUBSCRIPTIONS.may_load(deps.storage, &subscriber)? {
        Some(v) => v,
        None => return Ok(false),
    };

    let chargeable_amount = get_chargeable_amount(deps, &env, &subscription);

    match chargeable_amount {
        Ok(chargeable_amount) => {
            if chargeable_amount.amount.is_zero() {
                Ok(false)
            } else {
                // check if the subscription has lapsed
                let sub_active: bool = is_subscription_active(deps.storage, env, subscription);
                Ok(sub_active) // only return true if the subscription has not lapsed
            }
        }
        Err(_) => Ok(false),
    }
}

/// `query_subscriptions` returns all the subscriptions in the contract
/// caller can specify `start_after` and `limit` to paginate the responses
fn query_subscriptions(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<SubscriptionsResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let res: StdResult<Vec<SubscriptionInfo>> = SUBSCRIPTIONS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|elem| elem.map(|(_, subscription)| subscription))
        .collect();

    Ok(SubscriptionsResponse {
        subscriptions: res?,
    })
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = read_config(deps.storage)?;

    Ok(ConfigResponse {
        owner_address: config.owner_address.to_string(),
        receiver_address: config.receiver_address.to_string(),
        initial_amount: config.initial_amount,
        unit_interval_seconds: config.unit_interval.seconds(),
        unit_amount: config.unit_amount,
        additional_grace_period: config.additional_grace_period,
        paused: config.paused,
        uri: config.uri,
    })
}

/// is_subscription_active returns true if the subscription is active.
/// It takes into account the tolerance period that is set in the config
pub fn is_subscription_active(storage: &dyn Storage, env: Env, sub: SubscriptionInfo) -> bool {
    let config = read_config(storage).unwrap();

    // check if it is within the period
    match env.block.time <= sub.interval_end_at {
        true => true,
        false =>
        // check if it is within the grace period
        {
            match sub.is_cancelled {
                true => false, // users who cancelled do not fall within the grace period
                false => {
                    env.block.time
                        <= sub
                            .interval_end_at
                            .plus_seconds(DEFAULT_GRACE_PERIOD)
                            .plus_seconds(config.additional_grace_period)
                }
            }
        }
    }
}

/// query_subscription: returns the subscription status for a given subscriber
fn query_subscription(
    deps: Deps,
    env: Env,
    subscriber: String,
) -> StdResult<Option<SubscriptionInfoResponse>> {
    let response = match SUBSCRIPTIONS
        .may_load(deps.storage, &deps.api.addr_validate(&subscriber)?)?
    {
        Some(subscription) => {
            let config = read_config(deps.storage)?;

            let sub_active: bool =
                is_subscription_active(deps.storage, env.clone(), subscription.clone());
            let amount_chargeable: Uint256 = match sub_active {
                true => compute_amount_chargeable(&config, &subscription, env.block.time).amount,
                false => Uint256::zero(),
            };

            Some(SubscriptionInfoResponse {
                subscriber: subscription.owner.to_string(),
                created_at: subscription.created_at.seconds(),
                interval_end_at: subscription.interval_end_at.seconds(),
                last_charged: subscription.last_charged.seconds(),
                is_cancelled: subscription.is_cancelled,
                is_active: sub_active,
                discount_per_interval: subscription.discount_per_interval,
                amount_chargeable: Some(amount_chargeable),
            })
        }
        None => None,
    };

    Ok(response)
}
