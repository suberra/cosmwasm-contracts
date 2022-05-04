#  Fixed token stream  

A fixed-duration flow stream contract supporting native and cw20 tokens. 
- Each stream has a fixed start and end time.
- Amount streamed is evaluated on a seconds basis.

## Actions
1. **CreateStream**

Start a stream from start_at to end_at, allowing more tokens to be withdrawn by receiver over time.
```rust
CreateStream {
    receiver: String,
    token: Asset,
    start_at: u64,
    end_at: u64,
}
```
2. **Withdraw from stream**

Claim streamed tokens for the receiver. Omitting amount claims full balance in stream.
```rust
Withdraw {
    stream_id: u64,
    amount: Option<Uint128>,
}
```
3. **Cancel stream**

Deletes the stream, refunds unstreamed balance to both parties.
```rust
CancelStream {
    stream_id: u64,
}
```

## Queries

**1. Get Stream**

```rust
    Stream {
        stream_id: u64,
    }
```
JSON response:
```json
{
    "amount": "2000000",
    "created_at": 1642569458,
    "end_at": 1642629443,
    "from": "terra1zvpavp93m6p3rcrx9hlu6ldsqj4j4x6hcr9nw5",
    "interval": 600,
    "interval_due_at": 1642629458,
    "pending_charge": "0",
    "start_at": 1642569458,
    "status": "Expired",
    "to": "terra1rnxd25k3dahytvzxxdvakenaupkplvnjyvngv3"
}
```
**2. Balance of**

Returns the streamed balance (for receiver) or
Returns the unstreamed balance (for sender)

```rust
    BalanceOf {
        stream_id: u64,
        address: String,
    }
```
**3. Get list of stream**

```rust
    StreamsBySender {
        sender: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    StreamsByReceiver {
        receiver: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    StreamsByToken {
        token_info: AssetInfo,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    AllStreams {
        start_after: Option<u64>,
        limit: Option<u32>,
    }
```
JSON response:
```json
{
    "agreement_ids":
    [
        1,
        3,
        5,
        7,
        11,
        13,
        14,
        15,
        18,
        20
    ],
    "last_key": 20
}
```