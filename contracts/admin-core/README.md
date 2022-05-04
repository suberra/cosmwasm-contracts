# `admin-core`

Manages the separation of access control between the owner and the admins
* Owner: Highest-privilege access to the account. Can add and remove any admins and set allowance for all accounts. Can also spend without requiring approvals from anyone
* Admins: Can make most actions on the subwallet, but cannot add new admins nor give him or herself allowance to spend

This contract is adapted from `cw1-whitelist` contract.