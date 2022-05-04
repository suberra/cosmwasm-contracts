import * as fs from "fs";
import axios, { AxiosError } from "axios";
import chalk from "chalk";
import BN from "bn.js";
import 'dotenv/config';
import {
  isTxError,
  Coin,
  LocalTerra,
  Msg,
  MsgInstantiateContract,
  MsgStoreCode,
  MnemonicKey,
  StdFee,
  StdTx,
  Wallet,
  LCDClient,
  MsgMigrateContract,
} from "@terra-money/terra.js";
import {
  readFileSync,
  writeFileSync,
} from 'fs';
import path from 'path'
import { CustomError } from 'ts-custom-error'


export const ARTIFACTS_PATH = '../artifacts'

/**
 * @deprecated
 */
export function readArtifact(name: string = 'artifact') {
  try {
    const data = readFileSync(path.join(ARTIFACTS_PATH, `${name}.json`), 'utf8')
    return JSON.parse(data)
  } catch (e) {
    return {}
  }
}

/**
 * @deprecated
 */
export function writeArtifact(data: object, name: string = 'artifact') {
  writeFileSync(path.join(ARTIFACTS_PATH, `${name}.json`), JSON.stringify(data, null, 2))
}

export function readNetworkConfig(name: string) {
  if (!name) throw new Error("Error reading network config: name is required")
  try {
    const data = readFileSync(path.join(__dirname, `${name}.json`), 'utf8')
    return JSON.parse(data)
  } catch (e) {
    return {}
  }
}

export function writeNetworkConfig(data: object, name: string) {
  if (!name) throw new Error("Error writing network config: name is required")
  writeFileSync(path.join(__dirname, `${name}.json`), JSON.stringify(data, null, 2))
}


/**
 * @notice Encode a JSON object to base64 binary
 */
export function toEncodedBinary(obj: any) {
  return Buffer.from(JSON.stringify(obj)).toString("base64");
}

// /**
//  * @notice Send a transaction. Return result if successful, throw error if failed.
//  */
// export async function sendTransaction(
//   terra: LocalTerra | LCDClient,
//   sender: Wallet,
//   msgs: Msg[],
//   verbose = false
// ) {
//   const tx = await sender.createAndSignTx({
//     msgs,
//     fee: new StdFee(10000000, [new Coin("uusd", 45000000)]),
//   });

//   const result = await terra.tx.broadcast(tx);

//   // Print the log info
//   if (verbose) {
//     console.log(chalk.magenta("\nTxHash:"), result.txhash);
//     try {
//       console.log(
//         chalk.magenta("Raw log:"),
//         JSON.stringify(JSON.parse(result.raw_log), null, 2)
//       );
//     } catch {
//       console.log(chalk.magenta("Failed to parse log! Raw log:"), result.raw_log);
//     }
//   }

//   if (isTxError(result)) {
//     throw new Error(
//       chalk.red("Transaction failed!") +
//       `\n${chalk.yellow("code")}: ${result.code}` +
//       `\n${chalk.yellow("codespace")}: ${result.codespace}` +
//       `\n${chalk.yellow("raw_log")}: ${result.raw_log}`
//     );
//   }

//   return result;
// }


/* Utility to upload & init contract */
export async function deployContract(terra: LCDClient, wallet: Wallet, adminAddress: string, filepath: string, initMsg: object) {
  console.log("Deploying " + filepath);
  const codeId = await uploadContract(terra, wallet, filepath);
  return await instantiateContract(terra, wallet, adminAddress, codeId, initMsg);
}

/* Utility to upload */
export async function uploadContract(terra: LCDClient, wallet: Wallet, filepath: string) {
  const contract = readFileSync(filepath, 'base64');
  const uploadMsg = new MsgStoreCode(wallet.key.accAddress, contract);
  let result = await performTransaction(terra, wallet, [uploadMsg]);
  return Number(result.logs[0].eventsByType.store_code.code_id[0]) // code_id
}


/**
 * @notice Migrate contract code to new code_id
 */
export async function migrateContract(
  terra: LocalTerra | LCDClient,
  deployer: Wallet,
  contract: string,
  new_code_id: number,
  args?: object
) {
  return performTransaction(terra, deployer,
    [new MsgMigrateContract(deployer.key.accAddress, contract, new_code_id, args || {}),]
  );
}


export interface Client {
  wallet: Wallet
  terra: LCDClient | LocalTerra
}

export function recover(terra: LCDClient, mnemonic: string) {
  const mk = new MnemonicKey({ mnemonic: mnemonic });
  return terra.wallet(mk);
}


/**
 * 
 * @returns {terra, wallet} object upon isnstantiating a new client
 */

export function newClient(): Client {
  const client = <Client>{}
  if (process.env.WALLET) {
    client.terra = new LCDClient({
      URL: String(process.env.LCD_CLIENT_URL),
      chainID: String(process.env.CHAIN_ID)
    })
    client.wallet = recover(client.terra, process.env.WALLET)
  } else {
    client.terra = new LocalTerra()
    client.wallet = (client.terra as LocalTerra).wallets.test1
  }
  return client
}


/**
 * @notice Instantiate a contract from an existing code ID. Return contract address.
 */

export async function instantiateContract(terra: LCDClient, wallet: Wallet, admin_address: string, codeId: number, msg: object) {
  const instantiateMsg = new MsgInstantiateContract(wallet.key.accAddress, admin_address, codeId, msg, undefined);
  let result = await performTransaction(terra, wallet, [instantiateMsg])
  return result.logs[0].events[0].attributes.filter(element => element.key == 'contract_address').map(x => x.value)[0];
}


export async function performTransaction(terra: LCDClient, wallet: Wallet, msgs: Msg[]) {
  try {
    const tx = await createTransaction(wallet, msgs);
    const signedTx = await wallet.key.signTx(tx);

    const result = await broadcastTransaction(terra, signedTx)
    if (isTxError(result)) {
      throw new TransactionError(result.code, result.codespace, result.raw_log)
    }
    return result;
  } catch (err: any | AxiosError) {
    if (axios.isAxiosError(err)) {
      // Access to config, request, and response
      throw new Error(err.response?.data.error);
    } else {
      // Just a stock error
      throw err;
    }
  }
}

/**
 * @notice Return the native token balance of the specified account
 */
export async function queryNativeTokenBalance(
  terra: LocalTerra | LCDClient,
  account: string,
  denom: string = "uusd"
) {
  const balance = (await terra.bank.balance(account)).get(denom)?.amount.toString();
  if (balance) {
    return balance;
  } else {
    return "0";
  }
}

/**
 * @notice Return CW20 token balance of the specified account
 */
export async function queryTokenBalance(
  terra: LocalTerra | LCDClient,
  account: string,
  contract: string
) {
  const balanceResponse = await terra.wasm.contractQuery<{ balance: string }>(contract, {
    balance: { address: account },
  });
  return balanceResponse.balance;
}

/**
 * @notice Given a total amount of UST, find the deviverable amount, after tax, if we
 * transfer this amount.
 * @param amount The total amount
 * @dev Assumes a tax rate of 0.001 and cap of 1000000 uusd.
 * @dev Assumes transferring UST. Transferring LUNA does not incur tax.
 */
export function deductTax(amount: number) {
  const DECIMAL_FRACTION = new BN("1000000000000000000");
  const tax = Math.min(
    amount -
    new BN(amount)
      .mul(DECIMAL_FRACTION)
      .div(DECIMAL_FRACTION.div(new BN(1000)).add(DECIMAL_FRACTION))
      .toNumber(),
    1000000
  );
  return amount - tax;
}

/**
 * @notice Given a intended deliverable amount, find the total amount, including tax,
 * necessary for deliver this amount. Opposite operation of `deductTax`.
 * @param amount The intended deliverable amount
 * @dev Assumes a tax rate of 0.001 and cap of 1000000 uusd.
 * @dev Assumes transferring UST. Transferring LUNA does not incur tax.
 */
export function addTax(amount: number) {
  const tax = Math.min(new BN(amount).div(new BN(1000)).toNumber(), 1000000);
  return amount + tax;
}



export class TransactionError extends CustomError {
  public constructor(
    public code: number,
    public codespace: string | undefined,
    public rawLog: string,
  ) {
    super("transaction failed")
  }
}

let TIMEOUT = 2000

export function setTimeoutDuration(t: number) {
  TIMEOUT = t
}

export function getTimeoutDuration() {
  return TIMEOUT
}

export async function sleep(timeout: number) {
  await new Promise(resolve => setTimeout(resolve, timeout))
}

export async function createTransaction(wallet: Wallet, msgs: Msg[]) {
  return await wallet.createTx({ msgs, gasPrices: { 'uusd': 0.15 }, feeDenoms: ['uusd'] })
}

export async function broadcastTransaction(terra: LCDClient, signedTx: StdTx) {
  const result = await terra.tx.broadcast(signedTx)
  await sleep(TIMEOUT)
  return result
}