# Subwallet

Subwallet is a smart contract where users can hold their funds and transact. It provides the following functionalities:

- Hold any coins: Just like a normal wallet, Subwallet can hold any coin such as native tokens (LUNA, UST) or Cw-20 tokens
- Increase and decrease the allowance for coins on Subwallet: An owner or admin can change the allowance for the subwallet for any coins
- Freeze and unfreeze: Freeze and unfreeze subwallet, once frozen, no transactions can be processed
- Transfer aUST natively given a UST value without needing to withdraw and transfer.

## Roles and expected behavior

The are three main roles:

- Owner: the owner of the subwallet. Owner can do any actions on the wallet
- Admins: Admins can be added by the owner. Admins can do some privilege actions on the contract provided that the contract is not frozen
- Others: Any other addresses (contracts/EOAs) can perform actions only if it has been given permissions by the owner or admin

See table below for the difference in the roles and functions

| Actions             | Owner | Admins | Other           |
| ------------------- | ----- | ------ | --------------- |
| `IncreaseAllowance` | Yes   | Yes\*  | No              |
| `DecreaseAllowance` | Yes   | Yes\*  | No              |
| `TransferAToken`    | Yes   | Yes\*  | Only if granted |
| `Execute`           | Yes   | Yes\*  | Only if granted |
| `Freeze`            | Yes   | No     | No              |
| `Unfreeze`          | Yes   | No     | No              |
| `UpdateAdmins`      | Yes   | No     | No              |
| `SetPermissions`    | Yes   | No     | No              |

`*`: Only possible if the subwallet is not frozen by the owner

## Credits

Subwallet is an extended interface from `cw1-subkeys` from CosmWasm team. This contract is optimised to run on Terra blockchain.
