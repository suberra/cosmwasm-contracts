# `jobs-registry`

Handles jobs creation and deletion. Entrypoint for workers to discover new contracts to perform work on.

## ExecuteMsg

### AddJob

Adds a contract to the job-registry. Workers can only claim fees for upkeep on the contracts that have been added to the registry.

```json
{
  "add_job": {
    "contract_address": "terra1...",
    "name": "coolvideos!"
  }
}
```

### RemoveJob

Removes a job from the job registry. Can only be called by the address who have added the contract into the registry, or the admins.

```json
{
  "remove_job": {
    "contract_address": "terra1..."
  }
}
```

### WorkReceipt

Called when worker nodes complete a `Work` on the subscription contracts. Transfers credits available to the worker_address to reward for the work

```json
{
  "work_receipt": {
    "worker_address": "terra1..."
  }
}
```

### AddCredits

Add credits to the contract. Contract needs to have credits so that the workers can be paid. Returns an error if there's no jobs found for the particular contract.

```json
{
  "add_credits": {
    "contract_address": "terra1..."
  }
}
```

### SetBaseFee

Sets the base fee that can be claimable by the workers. Although the contract does not enforce a minimum base fee, the Base fees set by the admin should minimally cover the transaction cost on-chain otherwise the bots will lose money running the upkeep.

```json
{
  "set_base_fee": {
    "base_fee": [
      {
        "info": {
          "native_token": {
            "denom": "uusd"
          }
        },
        "amount": "1000000"
      }
    ]
  }
}
```
