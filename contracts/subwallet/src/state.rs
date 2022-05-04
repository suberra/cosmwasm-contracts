// use crate::allowance::PeriodicAllowance;
use crate::error::ContractError;
use cosmwasm_std::{Addr, Coin, Env, StdResult, Storage};
use cosmwasm_storage::{ReadonlySingleton, Singleton};
use cw0::{Expiration, NativeBalance};
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Sub;

// Permissions struct defines users message execution permissions.
// Could have implemented permissions for each cosmos module(StakingPermissions, GovPermissions etc...)
// But that meant a lot of code for each module. Keeping the permissions inside one struct is more
// optimal. Define other modules permissions here.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default, Copy)]
pub struct Permissions {
    pub delegate: bool,
    pub redelegate: bool,
    pub undelegate: bool,
    pub withdraw: bool,
}

impl fmt::Display for Permissions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "staking: {{ delegate: {}, redelegate: {}, undelegate: {}, withdraw: {} }}",
            self.delegate, self.redelegate, self.undelegate, self.withdraw
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct Allowance {
    pub balance: NativeBalance,
    pub expires: Expiration,
}

#[cfg(test)]
impl Allowance {
    /// Utility function for converting message to its canonical form, so two messages with
    /// different representation but same semantic meaning can be easily compared.
    ///
    /// It could be encapsulated in custom `PartialEq` implementation, but `PartialEq` is expected
    /// to be fast, so it seems to be reasonable to keep it as representation-equality, and
    /// canonicalize message only when it is needed
    ///
    /// Example:
    ///
    /// ```
    /// # use cw0::{Expiration, NativeBalance};
    /// # use cw1_subkeys::state::Allowance;
    /// # use cosmwasm_std::coin;
    ///
    /// let allow1 = Allowance {
    ///   balance: NativeBalance(vec![coin(1, "token1"), coin(0, "token2"), coin(2, "token1"), coin(3, "token3")]),
    ///   expires: Expiration::Never {},
    /// };
    ///
    /// let allow2 = Allowance {
    ///   balance: NativeBalance(vec![coin(3, "token3"), coin(3, "token1")]),
    ///   expires: Expiration::Never {},
    /// };
    ///
    /// assert_eq!(allow1.canonical(), allow2.canonical());
    /// ```
    pub fn canonical(mut self) -> Self {
        self.balance.normalize();
        self
    }
}

pub const PERMISSIONS: Map<&Addr, Permissions> = Map::new("permissions");
pub const ALLOWANCES: Map<&Addr, Allowance> = Map::new("allowances");
// pub const PERIODIC_ALLOWANCES: Map<&Addr, PeriodicAllowance> = Map::new("periodic_allowances");

// helper function to decrease the allowance for the user.]
pub fn deduct_allowance(
    storage: &mut dyn Storage,
    env: Env,
    spender: &Addr,
    coin: Vec<Coin>,
) -> Result<(), ContractError> {
    ALLOWANCES.update::<_, ContractError>(storage, spender, |allow| {
        let mut allowance = allow.ok_or(ContractError::NoAllowance {})?;
        if allowance.expires.is_expired(&env.block) {
            return Err(ContractError::NoAllowance {});
        }

        // Decrease allowance
        allowance.balance = allowance.balance.sub(coin.clone())?;
        Ok(allowance)
    })?;
    Ok(())
}

pub static CONFIG_KEY: &[u8] = b"config";
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner_addr: Addr,
    pub stable_denom: String,
    pub whitelist_contracts: Vec<Addr>,
    pub subwallet_factory_addr: Addr,
}

pub fn store_config(storage: &mut dyn Storage, data: &Config) -> StdResult<()> {
    Singleton::new(storage, CONFIG_KEY).save(data)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    ReadonlySingleton::new(storage, CONFIG_KEY).load()
}
