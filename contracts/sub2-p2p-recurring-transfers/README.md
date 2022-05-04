#  sub2-p2p-recurring-transfers  

Contracts for instantiating and managing peer-to-peer recurring transfers from a subwallet

## Actions

**1. Create Agreement**

```rust
CreateAgreement {
    receiver: String, // Receiver address
    amount: Uint256, // Amount to be transferred on each charge
    start_at: Option<u64>, // First charge start time, starts immediately if omitted
    end_at: Option<u64>, // End time, no charge can occur after this time
    interval: u64, // Interval duration in seconds
}
```

2. Cancel Agreement

Cancels and deletes agreement
```rust
CancelAgreement {
    agreement_id: u64,
}
```

3. Transfer

Transfer amount due in this agreement
```rust
Transfer {
    agreement_id: u64,
}
```


4. Work

Automation work unit, calls transfer internally
```rust
Work {
    payload: Binary,
}
```

5. Toggle Freeze

Toggles the `is_frozen` flag. If the previous value is true, it will set it to false. Vice-versa otherwise. 

```rust
ToggleFreeze {}
```

6. Toggle Pause 


Toggles the `is_paused` flag. If the previous value is true, it will set it to false. Vice-versa otherwise. 

```rust
TogglePause {}
```

## Queries

**1. Get agreement detail**

```rust
Agreement {
    agreement_id: u64,
}
```

**2. Get agreements**

All agreements
```rust
AllAgreements {
    start_after: Option<u64>,
    limit: Option<u32>,
}
```

By owner
```rust
AgreementsByOwner {
    owner: String,
    start_after: Option<u64>,
    limit: Option<u32>,
}
```

By receiver
```rust
AgreementsByReceiver {
    receiver: String,
    start_after: Option<u64>,
    limit: Option<u32>,
}
```

All overdued agreements
```rust
OverduedAgreements {
    start_after: Option<u64>, // u64 is interval_due_at time
    limit: Option<u32>,
}
```

**3. CanWork**
Automation helper
```rust
CanWork {
    payload: Binary,
}
```

**4. Config**

```rust
Config {},
```