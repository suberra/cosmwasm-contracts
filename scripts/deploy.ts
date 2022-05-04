import "dotenv/config";
import chalk from "chalk";

import {
  newClient,
  readNetworkConfig,
  writeNetworkConfig,
  Client,
  uploadContract,
  instantiateContract,
  performTransaction,
  sleep,
  toEncodedBinary,
} from "./helpers";
import { SuberraConfig, SuberraContractsConfig } from "./types";
import { Coin, MsgExecuteContract, MsgSend } from "@terra-money/terra.js";
import {
  createSubscribeMsgs,
  getCreatedSubscriptions,
  getIsSubscribed,
  getSubwallet,
  createRecurringTransferMsgs
} from "./suberra_sdk";

const ARTIFACTS_PATH = "../artifacts";

// Todo: add product_factory
// admin_core is omitted as it's imported within other contracts
// wasm artifact to name
const SUBERRA_CONTRACTS: {
  [file: string]: string
} = {
  jobs_registry: "jobs_registry",
  subwallet: "subwallet",
  product_factory: "product_factory",
  sub1_fixed_recurring_subscriptions: "subscription_product",
  sub2_p2p_recurring_transfers: "p2p",
  subwallet_factory: "subwallet_factory",
  token_stream: "token_stream",
};

//----------------------------------------------------------------------------------------
// utility functions
//----------------------------------------------------------------------------------------

export function readSuberraConfig(chainId: string): SuberraConfig {
  let config = readNetworkConfig("config");
  return config[chainId];
}

export function writeSuberraConfig(chainId: string, config: SuberraConfig): SuberraConfig {
  let currentConfig = readNetworkConfig("config");
  const newConfig = { ...currentConfig };
  newConfig[chainId] = config;
  writeNetworkConfig(newConfig, "config");
  return newConfig[chainId];
}

export function readSuberraContracts(chainId: string): SuberraContractsConfig {
  let config = readNetworkConfig(chainId);
  return {
    ...config,
  };
}

async function initContract(client: Client, name: string, initMsg: object) {
  console.log(
    `\tInitialising  ${name} with ${chalk.yellow(`msg: ${JSON.stringify(initMsg)}`)}`
  );
  const deployedState = readNetworkConfig(client.terra.config.chainID);
  if (deployedState[`${name}_contract`]) {
    console.log(
      chalk.gray("\t\tAlready Exists!"),
      `contractAddress = ${chalk.cyan(deployedState[`${name}_contract`])}`
    );
    return deployedState[`${name}_contract`];
  }
  if (!deployedState[`${name}_code_id`]) {
    console.log(chalk.red("\t\tCode not uploaded!"));
    return;
  }
  try {
    const codeId = deployedState[`${name}_code_id`];
    const contract = await instantiateContract(
      client.terra,
      client.wallet,
      client.wallet.key.accAddress,
      codeId,
      initMsg
    );
    console.log(chalk.green("\t\tDone!"), `contractAddress = ${chalk.cyan(contract)}`);
    deployedState[`${name}_contract`] = contract;
    writeNetworkConfig(deployedState, client.terra.config.chainID);
    return deployedState[`${name}_contract`];
  } catch (e) {
    console.log(chalk.red("\t\tError!"), e);
  }
}

//----------------------------------------------------------------------------------------
// Main
//----------------------------------------------------------------------------------------

(async () => {
  const client = newClient();

  console.log(chalk.yellow("\nSuberra Deployment"));

  console.log(`Network: ${chalk.cyan(client.terra.config.chainID)}`);
  console.log(`Using ${chalk.cyan(client.wallet.key.accAddress)} as deployer`);

  await uploadContracts(client);
  await initContracts(client);

  const contracts = readSuberraContracts(client.terra.config.chainID);
  console.log(chalk.blue("\tDeployed contracts:"), contracts);

  await linkContracts(client);

  // User & Merchants actions (comment out if you do not want to run the integration tests)
  // await createSubwallet(client);
  // await depositSubwallet(client);
  // await createSubscriptionProduct(client);
  // await subscribeToProduct(client);

  // create recurring transfer
  await createRecurringTransfer(client);
})();

//----------------------------------------------------------------------------------------
// Core functions
//----------------------------------------------------------------------------------------
async function uploadContracts(client: Client) {
  console.log(chalk.blue(`1. Storing contract code`));

  const deployedState = readNetworkConfig(client.terra.config.chainID);

  const isLocalTerra = client.terra.config.chainID == "localterra";
  if (isLocalTerra) {
    console.log(`\tAdding stub contracts for Localterra`);
    // Upload local artifacts instead of externally deployed ones
    SUBERRA_CONTRACTS["stub_anchor"] = "stub_anchor";
    SUBERRA_CONTRACTS["astroport_token"] = "aterra_token";
  }

  let file: keyof typeof SUBERRA_CONTRACTS;
  for (file in SUBERRA_CONTRACTS) {
    console.log(`\tUploading ${file} on ${client.terra.config.chainID}...`);

    const name = SUBERRA_CONTRACTS[file];
    if (!deployedState[`${name}_code_id`]) {
      try {
        const code_id = await uploadContract(
          client.terra,
          client.wallet,
          `${ARTIFACTS_PATH}/${file}.wasm`
        );

        deployedState[`${name}_code_id`] = code_id;
        console.log(chalk.green(`\t\t uploaded, code_id: ${code_id}`));
        writeNetworkConfig(deployedState, client.terra.config.chainID);
      } catch (e) {
        console.log(chalk.red(`\t\t failed to upload ${file}`), e);
        throw e;
      }
    } else {
      console.log(chalk.gray("\t\t already uploaded, skipped"));
    }
  }
}

async function initContracts(client: Client) {
  console.log(chalk.blue(`2. Init contracts`));

  await initContract(client, "jobs_registry", {});

  let config: SuberraConfig;
  const isLocalTerra = client.terra.config.chainID == "localterra";
  if (isLocalTerra) {
    // Init stubbed anchor money market
    const anchor_market_contract = await initContract(client, "stub_anchor", {});
    const aterra_token_contract = await initContract(client, "aterra_token", {
      name: "Local aUST",
      symbol: "laUST",
      decimals: 6,
      initial_balances: [
        { address: anchor_market_contract, amount: "1000000" }
      ],
      mint: {
        minter: anchor_market_contract
      }
    });

    console.log(`\tUpdating stubbed market contract...`);
    const tx = await performTransaction(client.terra, client.wallet, [
      new MsgExecuteContract(client.wallet.key.accAddress, anchor_market_contract, {
        update_config: {
          aterra_contract: aterra_token_contract
        },
      }),
    ]);
    console.log(chalk.yellow(`\t\t ${chalk.green(`Tx: ${tx.txhash}`)}`));

    config = writeSuberraConfig(client.terra.config.chainID, {
      anchor_market_contract,
      aterra_token_contract
    });

  } else {
    config = readSuberraConfig(client.terra.config.chainID);
  }
  console.log(`Attempting to upload subwallet_factory(1)`)

  const deployedState = readSuberraContracts(client.terra.config.chainID);
  console.log(deployedState);
  console.log(config);
  if (
    deployedState["subwallet_code_id"] &&
    config.anchor_market_contract &&
    config.aterra_token_contract
  ) {
    console.log(`Attempting to upload subwallet_factory`)
    await initContract(client, "subwallet_factory", {
      subwallet_code_id: deployedState["subwallet_code_id"],
      anchor_market_contract: config.anchor_market_contract,
      aterra_token_addr: config.aterra_token_contract,
    });
  } else {
    console.log("Missing subwallet_factory dependencies", deployedState["subwallet_code_id"], config.anchor_market_contract, config.aterra_token_contract)
  }


  // deploys the product subscription contract
  if (
    deployedState["subscription_product_code_id"] &&
    deployedState["jobs_registry_contract"]
  ) {
    await initContract(client, "product_factory", {
      product_code_id: deployedState["subscription_product_code_id"],
      protocol_fee_bps: 0, // 0% fee
      min_protocol_fee: "0", // $0
      min_amount_per_interval: "4000000", // $4
      min_unit_interval_hour: 24,
      fee_address: client.wallet.key.accAddress, // Send to self
      job_registry_address: deployedState["jobs_registry_contract"],
    });
  }

  // deploys the p2p contract
  if (deployedState["jobs_registry_contract"]) {
    await initContract(client, "p2p", {
      minimum_interval: 86400, // 1 day
      minimum_amount_per_interval: "10000000", // $10
      job_registry_contract: deployedState["jobs_registry_contract"],
      fee_bps: 0, // 0% fees
      // fee_address: client.wallet.key.accAddress, // omitted as it defaults to deployer
      max_fee: "1000000", // 1 usd
    });
  }

  await initContract(client, "token_stream", {});
}

async function linkContracts(client: Client) {
  const contracts = readSuberraContracts(client.terra.config.chainID);
  console.log(chalk.blue(`3. Linking contracts`));

  try {
    console.log(
      `\tAdding Job P2P: ${chalk.cyan(
        contracts.p2p_contract
      )} to registry ${chalk.cyan(contracts.jobs_registry_contract)}`
    );

    const tx = await performTransaction(client.terra, client.wallet, [
      new MsgExecuteContract(client.wallet.key.accAddress, contracts.jobs_registry_contract, {
        add_job: {
          contract_address: contracts.p2p_contract,
          name: "p2p",
        },
      }),
    ]);

    console.log(chalk.yellow(`\t\t ${chalk.green(`Tx: ${tx.txhash}`)}`));
  } catch (e: any) {
    console.log(chalk.red(`\t\t Failed to registry job, ${chalk.red(e?.rawLog || e)}`));
  }
}

async function createSubwallet(client: Client) {
  const contracts = readSuberraContracts(client.terra.config.chainID);
  console.log(chalk.blue(`4. Create subwallet`));

  try {
    const existing_subwallet = await getSubwallet(
      client.terra,
      contracts.subwallet_factory_contract,
      client.wallet.key.accAddress
    );

    if (existing_subwallet) {
      console.log(
        chalk.gray(
          `\t\t Subwallet ${chalk.cyan(existing_subwallet)} already exists, skipping`
        )
      );
      return;
    }

    console.log(
      `\tCreating subwallet for deployer: ${chalk.cyan(client.wallet.key.accAddress)}`
    );

    const tx = await performTransaction(client.terra, client.wallet, [
      new MsgExecuteContract(
        client.wallet.key.accAddress,
        contracts.subwallet_factory_contract,
        {
          create_account: {},
        }
      ),
    ]);

    console.log(`\t\t ${chalk.green(`Tx: ${tx.txhash}`)}`);
    sleep(1000); // delay for lcd to index
    const subwallet = await getSubwallet(
      client.terra,
      contracts.subwallet_factory_contract,
      client.wallet.key.accAddress
    );
    console.log(`\t\t Subwallet address = ${chalk.cyan(subwallet)}`);
  } catch (e: any) {
    console.log(
      chalk.red(`\t\t Failed to create subwallet, ${chalk.red(e?.rawLog || e)}`)
    );
  }
}

async function depositSubwallet(client: Client) {
  const contracts = readSuberraContracts(client.terra.config.chainID);
  const config = readSuberraConfig(client.terra.config.chainID);
  console.log(chalk.blue(`5. Deposit funds to subwallet`));
  const walletAddress = client.wallet.key.accAddress;

  try {
    const subwallet = await getSubwallet(
      client.terra,
      contracts.subwallet_factory_contract,
      client.wallet.key.accAddress
    );

    if (!subwallet) {
      console.log(chalk.gray(`\t\t No subwallet exists, skipping`));
      return;
    }

    // deposit $20 into the subwallet for testing
    const deposit_usd_amount = 20;
    console.log(
      `\t Depositing $${deposit_usd_amount} into subwallet: ${chalk.cyan(subwallet)}`
    );

    const coin = new Coin("uusd", deposit_usd_amount * 1e6).toIntCoin();
    const transfer = new MsgSend(walletAddress, subwallet, [coin]);

    const anchorDeposit = new MsgExecuteContract(walletAddress, subwallet, {
      execute: {
        msgs: [
          {
            wasm: {
              execute: {
                funds: [coin.toData()],
                contract_addr: config.anchor_market_contract,
                msg: toEncodedBinary({
                  deposit_stable: {},
                }),
              },
            },
          },
        ],
      },
    });

    const tx = await performTransaction(client.terra, client.wallet, [
      transfer,
      anchorDeposit,
    ]);

    console.log(`\t\t Deposited! ${chalk.green(`Tx: ${tx.txhash}`)}`);
  } catch (e: any) {
    console.log(
      chalk.red(`\t\t Failed to deposit into subwallet, ${chalk.red(e?.rawLog || e)}`)
    );
  }
}

async function createSubscriptionProduct(client: Client) {
  const contracts = readSuberraContracts(client.terra.config.chainID);
  console.log(chalk.blue(`5. Create subscription product`));

  try {
    const existing_products = await getCreatedSubscriptions(
      client.terra,
      contracts.product_factory_contract,
      client.wallet.key.accAddress
    );

    if (existing_products && existing_products.length) {
      console.log(
        chalk.gray(
          `\t\t Product ${chalk.cyan(existing_products)} already exists, skipping`
        )
      );
      return;
    }

    console.log(
      `\tCreating product for deployer: ${chalk.cyan(client.wallet.key.accAddress)}`
    );

    const tx = await performTransaction(client.terra, client.wallet, [
      new MsgExecuteContract(
        client.wallet.key.accAddress,
        contracts.product_factory_contract,
        {
          create_product: {
            product_info: {
              receiver_address: client.wallet.key.accAddress,
              unit_amount: "10000000", // $10
              initial_amount: "1000000", // $10
              unit_interval_hour: 1,
              additional_grace_period_hour: 1,
              uri: "",
              admins: [client.wallet.key.accAddress],
              mutable: true,
            },
          },
        }
      ),
    ]);

    console.log(`\t\t ${chalk.green(`Tx: ${tx.txhash}`)}`);
    sleep(1000); // delay for lcd to index
    const products = await getCreatedSubscriptions(
      client.terra,
      contracts.product_factory_contract,
      client.wallet.key.accAddress
    );
    console.log(`\t\t Products address = ${chalk.cyan(products)}`);
  } catch (e: any) {
    console.log(chalk.red(`\t\t Failed to create product, ${chalk.red(e?.rawLog || e)}`));
  }
}

async function subscribeToProduct(client: Client) {
  const contracts = readSuberraContracts(client.terra.config.chainID);
  console.log(chalk.blue(`6. Subscribe to subscription product`));

  try {
    const existing_products = await getCreatedSubscriptions(
      client.terra,
      contracts.product_factory_contract,
      client.wallet.key.accAddress
    );

    if (!existing_products || existing_products.length == 0) {
      console.log(chalk.gray(`\t\t No product exists, skipping`));
      return;
    }

    const subscription_product = existing_products?.[0];

    const subwallet = await getSubwallet(
      client.terra,
      contracts.subwallet_factory_contract,
      client.wallet.key.accAddress
    );

    if (!subwallet) {
      console.log(chalk.gray(`\t\t No subwallet exists, skipping`));
      return;
    }

    const isSubscribed = await getIsSubscribed(
      client.terra,
      subscription_product,
      subwallet
    );

    if (isSubscribed) {
      console.log(chalk.gray(`\t\t Subwallet already subscribed, skipping`));
      return;
    }

    console.log(`\t Subscribing to product: ${chalk.cyan(subscription_product)}`);
    console.log(`\t\t from deployer's subwallet: ${chalk.cyan(subwallet)}`);

    const tx = await performTransaction(
      client.terra,
      client.wallet,
      createSubscribeMsgs(client.wallet.key.accAddress, subwallet, subscription_product)
    );

    console.log(`\t\t Subscribed! ${chalk.green(`Tx: ${tx.txhash}`)}`);
  } catch (e: any) {
    console.log(chalk.red(`\t\t Failed to subscribe, ${chalk.red(e?.rawLog || e)}`));
  }
}


async function createRecurringTransfer(client: Client) {
  const contracts = readSuberraContracts(client.terra.config.chainID);
  console.log(chalk.blue(`7. Create recurring transfer`));

  try {

    const subwallet = await getSubwallet(
      client.terra,
      contracts.subwallet_factory_contract,
      client.wallet.key.accAddress
    );


    if (!subwallet) {
      console.log(chalk.gray(`\t\t No subwallet exists, skipping`));
      return;
    }

    const p2pContract = contracts.p2p_contract;

    console.log(`\t Attempting to create recurring transfer`);

    const tx = await performTransaction(
      client.terra,
      client.wallet,
      createRecurringTransferMsgs(client.wallet.key.accAddress, subwallet, p2pContract, "terra1qqr3cq5l7d85vtm0fvpt795urf9xujhyc88kw7")

    );

    console.log(`\t\t Recurring transfer created! ${chalk.green(`Tx: ${tx.txhash}`)}`);
    console.log(tx.logs);


  } catch (e: any) {
    console.log(chalk.red(`\t\t Failed to create recurring transfer:${chalk.red(e?.rawLog || e)}`));
  }

}
