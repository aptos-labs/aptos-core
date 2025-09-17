// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Result};
use aptos_language_e2e_tests::account::Account;
use move_core_types::account_address::AccountAddress;
use move_package::source_package::parsed_manifest::NamedAddress;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
};

/// Represents a unique user account
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct UserId(usize);

/// Types of named address
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum NamedAddressKind {
    /// first-party item
    Primary,
    /// third-party item
    Dependency,
    /// Aptos framework
    Framework,
}

impl Display for NamedAddressKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primary => write!(f, "Primary"),
            Self::Dependency => write!(f, "Dependency"),
            Self::Framework => write!(f, "Framework"),
        }
    }
}

/// Details about the nature of an address
#[derive(Debug, Clone)]
pub enum AddressDetails {
    /// a named address declared in Move package manifest
    Named {
        kind: NamedAddressKind,
        names: BTreeSet<NamedAddress>,
        account: Account,
    },

    /// an address that represents a normal user who interacts with the system
    User { id: UserId, account: Account },
}

impl Display for AddressDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Named {
                kind,
                names,
                account: _,
            } => write!(
                f,
                "{kind}({})",
                names
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Self::User { id, account: _ } => write!(f, "User({})", id.0),
        }
    }
}

/// Type of this address
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum AddressKind {
    Named(NamedAddressKind),
    User,
}

/// Address registry
#[derive(Debug, Clone)]
pub struct AddressRegistry {
    /// mapping from address to address/account details
    details: BTreeMap<AccountAddress, AddressDetails>,

    /// mapping from named address (i.e., string symbols) to address
    named_addresses: BTreeMap<NamedAddress, AccountAddress>,
    /// mapping from user id to address
    user_addresses: BTreeMap<UserId, AccountAddress>,
}

impl AddressRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self {
            details: BTreeMap::new(),
            named_addresses: BTreeMap::new(),
            user_addresses: BTreeMap::new(),
        }
    }

    /// Report a `(name, addr)` pair to the registry
    /// - If the pair does not exist, create an account and insert it
    /// - If `addr` exists but `name` is new, link them after validation
    /// - If `name` exists but `addr` is new, this is definitely an error
    /// - If both exist, check `addr` if needed
    ///
    /// Return the account if new account is created, `None` otherwise.
    pub fn sync_named_address(
        &mut self,
        name: NamedAddress,
        addr: AccountAddress,
        kind_to_check_on_exists: Option<NamedAddressKind>,
        kind_to_insert_on_empty: NamedAddressKind,
    ) -> Result<Option<Account>> {
        let account_created = match (self.named_addresses.get(&name), self.details.get_mut(&addr)) {
            (None, None) => {
                let names = std::iter::once(name).collect();
                // NOTE: here it means all accounts created by our executor
                // share the same key pair, but that seems okay.
                let account = Account::new_genesis_account(addr);
                self.details.insert(addr, AddressDetails::Named {
                    kind: kind_to_insert_on_empty,
                    names,
                    account: account.clone(),
                });
                self.named_addresses.insert(name, addr);
                log::debug!("named address {name}: {addr} registered as {kind_to_insert_on_empty}");

                // mark that new account is created
                Some(account)
            },
            (
                None,
                Some(AddressDetails::Named {
                    kind,
                    names,
                    account: _,
                }),
            ) => {
                match kind_to_check_on_exists {
                    Some(expected) if expected != *kind => {
                        bail!("expect {name}:{addr} to be {expected}, found {kind}");
                    },
                    _ => (),
                }

                let inserted = names.insert(name);
                assert!(inserted);
                self.named_addresses.insert(name, addr);
                log::debug!("named address {name}: {addr} associated as {kind}");

                // no new account created
                None
            },
            (None, Some(details)) => {
                bail!("expecting {addr} to be a named address, found '{details}'");
            },
            (Some(previous), None) => bail!(
                "conflicting assignment for named address {name}: {addr}, \
                 {name} is already bound to {previous} and {addr} does not exist"
            ),
            (Some(previous), Some(details)) => {
                if previous != &addr {
                    bail!(
                        "conflicting assignment for named address {name}: {addr}, \
                         {name} is already bound to {previous} and {addr} is {details}"
                    );
                }
                match details {
                    AddressDetails::Named {
                        kind,
                        names,
                        account: _,
                    } => {
                        match kind_to_check_on_exists {
                            Some(expected) if expected != *kind => {
                                bail!("expect {name}:{addr} to be {expected}, found {kind}");
                            },
                            _ => (),
                        }
                        assert!(names.contains(&name));
                    },
                    _ => bail!("expecting {addr} to be a named address, found '{details}'"),
                }

                // no new account created
                None
            },
        };
        Ok(account_created)
    }

    /// Create a new user account and return its address
    pub fn make_user_account(&mut self) -> Account {
        let id = UserId(self.user_addresses.len());
        let address = stable_user_address(id);
        let account = Account::new_genesis_account(address);

        let exists = self.user_addresses.insert(id, address);
        assert!(exists.is_none());
        assert!(!self.details.contains_key(&address));
        self.details.insert(address, AddressDetails::User {
            id,
            account: account.clone(),
        });

        account
    }

    /// Lookup the account from an address
    pub fn lookup_account(&self, addr: AccountAddress) -> Option<&Account> {
        match self.details.get(&addr)? {
            AddressDetails::Named { account, .. } | AddressDetails::User { account, .. } => {
                Some(account)
            },
        }
    }

    /// Return all addresses stored in this registry, sorted by kind
    pub fn all_addresses_by_kind(&self) -> BTreeMap<AddressKind, BTreeSet<AccountAddress>> {
        let mut mapping: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
        for (addr, details) in &self.details {
            let inserted = match details {
                AddressDetails::Named { kind, .. } => mapping
                    .entry(AddressKind::Named(*kind))
                    .or_default()
                    .insert(*addr),
                AddressDetails::User { .. } => {
                    mapping.entry(AddressKind::User).or_default().insert(*addr)
                },
            };
            assert!(inserted);
        }
        mapping
    }
}

fn stable_user_address(id: UserId) -> AccountAddress {
    let mut bytes = [0u8; AccountAddress::LENGTH];
    bytes[0] = 0xFD;
    bytes[AccountAddress::LENGTH - std::mem::size_of::<usize>()..]
        .copy_from_slice(&id.0.to_be_bytes());
    AccountAddress::new(bytes)
}

#[cfg(test)]
mod tests {
    use super::{AddressKind, AddressRegistry, NamedAddressKind};
    use anyhow::Result;
    use move_core_types::account_address::AccountAddress;
    use move_package::source_package::parsed_manifest::NamedAddress;

    fn named(name: &str) -> NamedAddress {
        name.into()
    }

    #[test]
    fn test_sync_named_address_creates_and_aliases_account() -> Result<()> {
        let mut registry = AddressRegistry::new();
        let addr = AccountAddress::from_hex_literal("0x123")?;

        let created =
            registry.sync_named_address(named("primary"), addr, None, NamedAddressKind::Primary)?;
        assert!(created.is_some());

        let aliased = registry.sync_named_address(
            named("alias"),
            addr,
            Some(NamedAddressKind::Primary),
            NamedAddressKind::Primary,
        )?;
        assert!(aliased.is_none());

        let mapping = registry.all_addresses_by_kind();
        assert_eq!(
            mapping.get(&AddressKind::Named(NamedAddressKind::Primary)),
            Some(&std::collections::BTreeSet::from([addr]))
        );
        Ok(())
    }

    #[test]
    fn test_sync_named_address_rejects_conflicting_assignment() -> Result<()> {
        let mut registry = AddressRegistry::new();
        let addr_a = AccountAddress::from_hex_literal("0x1")?;
        let addr_b = AccountAddress::from_hex_literal("0x2")?;
        registry.sync_named_address(named("shared"), addr_a, None, NamedAddressKind::Primary)?;

        let err = registry
            .sync_named_address(named("shared"), addr_b, None, NamedAddressKind::Primary)
            .unwrap_err();
        assert!(err.to_string().contains("conflicting assignment"));
        Ok(())
    }

    #[test]
    fn test_make_user_account_registers_user_kind() {
        let mut registry = AddressRegistry::new();
        let user_a = registry.make_user_account();
        let user_b = registry.make_user_account();

        assert_ne!(user_a.address(), user_b.address());
        let mapping = registry.all_addresses_by_kind();
        let users = mapping.get(&AddressKind::User).unwrap();
        assert!(users.contains(&user_a.address()));
        assert!(users.contains(&user_b.address()));
        assert_eq!(users.len(), 2);
    }

    #[test]
    fn test_make_user_account_is_deterministic_across_registries() {
        let mut left = AddressRegistry::new();
        let mut right = AddressRegistry::new();

        let left_a = left.make_user_account();
        let right_a = right.make_user_account();
        let left_b = left.make_user_account();
        let right_b = right.make_user_account();

        assert_eq!(left_a.address(), right_a.address());
        assert_eq!(left_b.address(), right_b.address());
        assert_ne!(left_a.address(), left_b.address());
    }
}
