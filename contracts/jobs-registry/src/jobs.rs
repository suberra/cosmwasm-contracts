use crate::msg::AllJobsResponse;
use crate::msg::JobInfo;
use crate::state::JOBS;
use cosmwasm_std::Deps;
use cosmwasm_std::{Order, StdResult};
use cw_storage_plus::Bound;

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

fn calc_limit(request: Option<u32>) -> usize {
    request.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize
}

// return a list of all jobs here
pub fn query_all_jobs(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<AllJobsResponse> {
    let limit = calc_limit(limit);
    let start = start_after.map(Bound::exclusive);

    let res: StdResult<Vec<JobInfo>> = JOBS
        .range(deps.storage, start, None, Order::Ascending)
        .filter(|item| {
            if let Ok((_, job)) = item {
                job.active
            } else {
                false
            }
        })
        .take(limit)
        .map(|item| item.map(|(_, job)| JobInfo::from(job)))
        .collect();
    Ok(AllJobsResponse { jobs: res? })
}
