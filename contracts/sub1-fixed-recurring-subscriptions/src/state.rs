use crate::error::ContractError;
use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{Addr, StdResult, Storage, Timestamp};
use cosmwasm_storage::{bucket_read, ReadonlySingleton, Singleton};
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use suberra_core::subscriptions::Discount;

const CONFIG_KEY: &[u8] = b"config";
const PREFIX_SUBSCRIPTIONS: &[u8] = b"subscriptions";

/// Takes in a [`SubscriptionInfo`] object and save it in the Subscriptions application.
/// Increments the counter by one.
pub fn create_subscription(
    storage: &mut dyn Storage,
    subscriber: Addr,
    subscription: SubscriptionInfo,
) -> Result<(), ContractError> {
    SUBSCRIPTIONS.save(storage, &subscriber, &subscription)?;
    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner_address: Addr,
    pub receiver_address: Addr,
    pub additional_grace_period: u64,
    pub unit_interval: Timestamp,
    pub unit_amount: Uint256,
    pub initial_amount: Uint256,
    pub paused: bool,
    pub uri: String,
    pub factory_address: Addr,
}

/// # Description
/// Stores the metadata about every subscription object per user
///- created_at: timestamp when the subscription was first started
/// - last_charged: last_charged timestamp
/// - interval_end_at : timestamp where the subscription for the subscriber will be valid till
/// - discount (optional): Discount applicable for the subscriber per interval
/// - is_cancelled: Returns a value on type [`bool`] on whether the subscription is cancelled
/// - owner: Value of type [`Addr`] of the owner of the object (i.e. the Subscriber)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SubscriptionInfo {
    pub created_at: Timestamp,
    pub last_charged: Timestamp,
    pub interval_end_at: Timestamp,
    pub discount_per_interval: Option<Discount>,
    pub is_cancelled: bool,
    pub owner: Addr,
}

// Saves the Subscriptions metadata for all subscribers
pub const SUBSCRIPTIONS: Map<&Addr, SubscriptionInfo> = Map::new("subscriptions");

/// Saves the config of type [`Config`]
pub fn store_config(storage: &mut dyn Storage, data: &Config) -> StdResult<()> {
    Singleton::new(storage, CONFIG_KEY).save(data)
}

/// Reads the [`Config`] file of the user
pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    ReadonlySingleton::new(storage, CONFIG_KEY).load()
}

/// Reads the information of a user and returns an object of [`Option<SubscriptionInfo>`] given a `Subscriber` of type [`Addr`].
/// If the user cannot be found, returns `None`
pub fn read_subscription_info(
    storage: &dyn Storage,
    subscriber: &Addr,
) -> Option<SubscriptionInfo> {
    match bucket_read(storage, PREFIX_SUBSCRIPTIONS).load(subscriber.as_bytes()) {
        Ok(v) => v,
        _ => None,
    }
}
