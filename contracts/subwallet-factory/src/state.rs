use cosmwasm_std::{Addr, StdResult, Storage};
use cosmwasm_storage::{bucket, bucket_read, ReadonlySingleton, Singleton};
use suberra_core::subwallet_factory::SubwalletFactoryConfig as Config;

pub static CONFIG_KEY: &[u8] = b"config";
pub static ACCOUNTS_KEY: &[u8] = b"accounts";

pub fn store_config(storage: &mut dyn Storage, data: &Config) -> StdResult<()> {
    Singleton::new(storage, CONFIG_KEY).save(data)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    ReadonlySingleton::new(storage, CONFIG_KEY).load()
}

// stores the mapping between user address and its subwallet
pub fn store_address(storage: &mut dyn Storage, owner: &Addr, subwallet: &Addr) -> StdResult<()> {
    bucket(storage, ACCOUNTS_KEY).save(owner.as_bytes(), subwallet)?;

    Ok(())
}

pub fn retrieve_address(storage: &dyn Storage, owner: &Addr) -> Option<Addr> {
    match bucket_read(storage, ACCOUNTS_KEY).load(owner.as_bytes()) {
        Ok(v) => Some(v),
        _ => None,
    }
}
