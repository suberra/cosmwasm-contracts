use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AdminConfig {
    pub owner: Addr,
    pub admins: Vec<Addr>,
    pub mutable: bool,
}

impl AdminConfig {
    pub fn is_owner(&self, addr: &str) -> bool {
        self.owner == addr
    }

    pub fn is_admin(&self, addr: &str) -> bool {
        self.admins.iter().any(|a| a.as_ref() == addr)
    }

    /// returns true if the address is a registered admin and the config is mutable
    pub fn can_modify_as_admin(&self, addr: &str) -> bool {
        self.mutable && self.is_admin(addr)
    }

    pub fn can_change_admins(&self, addr: &str) -> bool {
        self.mutable && self.is_owner(addr)
    }
}

pub const ADMIN_CONFIG: Item<AdminConfig> = Item::new("admin_config");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_admin() {
        let admins: Vec<_> = vec!["bob", "paul", "john"]
            .into_iter()
            .map(Addr::unchecked)
            .collect();
        let config = AdminConfig {
            owner: Addr::unchecked("owner"),
            admins: admins.clone(),
            mutable: false,
        };

        assert!(config.is_admin(admins[0].as_ref()));
        assert!(config.is_admin(admins[2].as_ref()));
        assert!(!config.is_admin("other"));
    }

    #[test]
    fn can_modify() {
        let alice = Addr::unchecked("alice");
        let bob = Addr::unchecked("bob");

        // admin can modify mutable contract
        let config = AdminConfig {
            owner: Addr::unchecked("owner"),
            admins: vec![bob.clone()],
            mutable: true,
        };
        assert!(!config.can_modify_as_admin(alice.as_ref()));
        assert!(config.can_modify_as_admin(bob.as_ref()));

        // no one can modify an immutable contract
        let config = AdminConfig {
            owner: Addr::unchecked("owner"),
            admins: vec![alice.clone()],
            mutable: false,
        };
        assert!(!config.can_modify_as_admin(alice.as_ref()));
        assert!(!config.can_modify_as_admin(bob.as_ref()));
    }
}
