# Product Factory
Product factory stores all the products and also allow a user to instantiate a product subscription contract. 

Product subscription contracts instantiated through this contract will read the settings from the factory, including fee information such as the `protocol_fee_bps` (Protocol fee in basis points) and `protocol_fee_address`. 

## ExecuteMsg

### `create_product`
Anyone can create a product. When a user executes this operation, it creates a Product Subscription contract.  

```json
{
    "create_product" : {
        "product_info": {
            "receiver_address": "terra14q93cklzvskelyyq85s7x38rsmhemvwft2t27q",
            "unit_amount" : "4000000",
            "initial_amount" : "4000000",
            "unit_interval_hour" : 24,
            "max_amount_chargeable" : "4000000",
            "additional_grace_period_hour" : 48,
            "uri": "https://metadata.link/json",
            "admins" : [],
            "mutable": false
        }
    }
}
```

### `update_config`
Updates the config of the product factory. Any value can be changed, but is subjected to validation checks.

```json
{
    "update_config" : {
        "new_owner" : "terra1...",
        "new_product_code_id": 5,
        "new_protocol_fee_bps" : "100",
        "new_fee_address" : "terra1fee",
        "new_job_registry" : null,
    }
}

```


## QueryMsg

All query messages are described below. A custom struct is defined for each query response.

### `config`

Returns general settings of the factory

```json
{
  "config": {}
}
```