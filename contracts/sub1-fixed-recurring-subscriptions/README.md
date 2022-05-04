# sub1-fixed-recurring-subscriptions

Contract for managing fixed-period recurring payments. Stores subscriber information and enforces payment rules.

--

## InstantiateMsg

- receiver_address: address that will receive the payments
- job_registry_contact: Contract address of the job_registry. Required for automation
- unit_amount: Amount to be charged in every billing cycle
- initial_amount: initial_amount that must be transferred to the receiver for the subscription to be created. Common in most services
- unit_interval_hour: Duration of the billing cycle in hours
- max_amount_chargeable: Maximum amount that will be chargeable to the subscriber.
- additional_grace_period_hour: Amount of time (in hours) that a subscription should still be active despite payment is due
- uri : Metadata for the subscription
- admins: List of admins that have the rights to manage some features of the product contracts
- mutable: States if the contract is mutable
- factory_address: Stores the address of the factory that instantiates the contract

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ProductInstantiateMsg {
    pub receiver_address: String,
    pub job_registry_contract: Option<String>,
    pub unit_amount: Uint256,
    pub initial_amount: Uint256,
    pub unit_interval_hour: u64,
    pub max_amount_chargeable: Option<Uint256>,
    pub additional_grace_period_hour: Option<u64>,
    pub uri: String,
    pub admins: Vec<String>,
    pub mutable: bool,
    pub factory_address: String,
}
```

## State

subscriber address -> Subscription

```rust
pub const SUBSCRIPTIONS: Map<&Addr, SubscriptionInfo> = Map::new("subscriptions");

pub struct SubscriptionInfo {
    pub created_at: Timestamp,
    pub last_charged: Timestamp,
    pub valid_until: Timestamp,
    pub discount: Option<Uint256>,
    pub is_cancelled: bool,
    pub owner: Addr,
}
```

## Actions

**1. Subscribe**

Called via a subwallet to subscribe to a service.
This requires aUST allowance to be approved beforehand.

**2. Cancel**

Cancels a subscription service, subscription status will still be active until cycle ends.
Prevents further charges to be made.

**3. Charge**
Charge a particular payer who's subscription payment is dued.

```rust
Charge {
    // Subwallet address of the payer
    payer_address: String,
}
```

## Queries

**1. Get subscription detail **

```rust
Subscription {
    subscriber: String,
}
```

Response:

```rust
pub struct SubscriptionInfoResponse {
    pub subscriber: String,
    pub created_at: u64, // unix timestamp for when subcriber subscribed at
    pub valid_until: u64, // unix timestamp for next cycle end
    pub last_charged: u64, // unix timestamp for last successful charge
    pub is_cancelled: bool,
    pub is_active: bool,
    pub discount: Option<Uint256>,
    pub amount_chargeable: Option<Uint256>, // Pending charge amount
}
```
