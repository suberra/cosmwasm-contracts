use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use sub1_fixed_recurring_subscriptions::msg::{
    ConfigResponse, ExecuteMsg, QueryMsg, SubscriptionInfoResponse, SubscriptionsResponse,
};
use sub1_fixed_recurring_subscriptions::state::Config;
use suberra_core::msg::ProductInstantiateMsg;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(ProductInstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(Config), &out_dir);
    export_schema(&schema_for!(SubscriptionInfoResponse), &out_dir);
    export_schema(&schema_for!(ConfigResponse), &out_dir);
    export_schema(&schema_for!(SubscriptionsResponse), &out_dir);
}
