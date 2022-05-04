# CHANGELOG

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# 0.2.3 

## Added
* `subwallet_factory`: Added `UpdateConfig`
* `product_factory`: Added minimum unit interval hours
* `job_registry`: Added admins, which are a set of addresses that can add jobs to the job registry contracts
* `sub1-fixed-recurring-subscriptions`: Freeze and Pause feature
* `sub2-p2p-recurring-transfers`: Freeze and Pause feature
* 
### Changed
* `sub1` and `sub2`: `can_work` now returns a false if the contract is frozen
* `token-stream`: Allow anyone to withdraw/settle a stream

# 0.2.2 (Audited)
* `sub2-p2p-recurring-payments`: Added `last_charged` timestamp to p2p agreements

# 0.2.1

### Added
* `subwallet`: Freeze feature which prevents `IncreaseAllowance`, `DecreaseAllowance`, `TransferAToken` and `Execute`
  from being called on the subwallet by a non-owner.
* `admin-core`: Added `Unfreeze` function to resume transactions
* `admin-core`: Limited the number of admins to a maximum of 10. 
* `sub1-fixed-recurring-subscriptions`: Return error if subscription cannot be found.
* `product-factory`: Added `min_amount_per_interval` into config, and checks for minimum amount during product creation
* `sub2-p2p-recurring-payments`: Added minimum amount per interval for p2p recurring payments

### Changed
* `sub1-fixed-recurring-subscriptions`: Changed `valid_until` to `interval_end_at`
* `sub1-fixed-recurring-subscriptions`: Prevent charges after the subscription expires
* `sub1-fixed-recurring-subscriptions`: Queries the factory to fetch `job_registry` contract from product factory
* `sub1-fixed-recurring-subscriptions`: Increased `default_grace_period` from 23 to 24 hours
* `subwallet`: Queries the factory to fetch `aterra_token_addr` and `anchor_market_contract` from the factory address
* `admin-core`: Changed `AdminList` to `AdminConfig`, and returned `owner` on the `AdminConfigResponse`
* `token-stream`: Allows any stream amount (instead of requiring multiples of duration)
* `token-stream`: Made start_at optional (starts immediately if omitted)

### Fixed
* `sub1-fixed-recurring-subscriptions`: Fixed the error which causes incorrect amount to be charged should the subscriber misses two intervals.
* `sub1-fixed-recurring-subscriptions`: Fixed incorrect max protocol fee decimal places
* `sub1-fixed-recurring-subscriptions`: Removed subscription counter as it does not reflect correct subscription count due to cancellation and potential lapses in subscriptions
* `subwallet`: Owner can now set permissions for `execute_set_permission`
* `token-stream`: Fixed 0 amount error when cancelling streams. 

# 0.2.0

Version that is sent for code audits.

### Added
* `deploy.ts`: All in one deployment script
* packages:suberra_core: added more interfaces and common interfaces to packages
* `product-factory`: Factory contract that handles product subscriptions instantiation and fee info management
* `product-factory`: Added minimum fees
  
### Removed

* `deploy-*.ts`: Cleaned up old deployment scripts

### Changed

* `factory`: Renamed to `subwallet-factory`
* `subwallet-factory`: Refactor to use submessages

# 0.1.4

### Added

* `token-stream`: Allow users to stream cw20 or native token to recipient over time

### Changed

* `subwallet-admin`: Renamed to `admin-core` as it's used as a base contract for multiple contracts
* `p2p-recurring-transfers`: Refactor to use incremental agreement_id as pk


# 0.1.3

### Added
* Subscriptions: Allow owner to modify the `valid_until`, `last_charged` and `created_at` timestamp of any existing subscriber

## 0.1.2

### Changed

- Changed logic to charge based on `valid_until` field in Subscriptions
- Added `valid_until` field in Subscription
- Fixed error in charging

## 0.1.1

### Added

- `subwallet-admin`: Added `QueryMsg::Owner{}` which returns the owner of the subwallet

### Changed

- `sub1-fixed-recurring-subscriptions`: Emits `module_contract_address` for better off-chain logging.
