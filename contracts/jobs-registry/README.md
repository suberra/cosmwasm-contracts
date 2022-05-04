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

### UpdateAdmins

Updates the admins for the job registry contract. The admins are able to add jobs to the contract. Can only be called by the owner. Total number of possible admins is capped at 10.

```json
{
  "update_admins": {
    "admins": [
      "terra16z3zc0dv7hfg46falyrv7vhuf2dtyr747ds5yh",
      "terra1falyr6z3zc0dv7hfg46vtyr747ds7yh7vhuf2d"
    ]
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


### UpdateAdmins
Update admins. Only admins can add a job to the registry

```
{
    "update_admins": {
        "admins": ["terra1eqrnp2h43u0s7nssv6f68ccmdpywdaz2y76yee"]
    }
}
```