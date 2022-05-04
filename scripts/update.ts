import { MsgExecuteContract } from "@terra-money/terra.js";
import chalk from "chalk";
import { Client, newClient, performTransaction } from "./helpers";

(async function main() {
  const client = newClient();

  console.log(chalk.yellow("\nSuberra Update Metadata"));

  console.log(`Network: ${chalk.cyan(client.terra.config.chainID)}`);
  console.log(`Using ${chalk.cyan(client.wallet.key.accAddress)} as deployer`);

  const uri_base =
    "https://whdejulpbgvgeyrtprzn.supabase.in/storage/v1/object/public/subscription-meta/<contract>.json";
  const contracts = [
    "terra19gd2v6reuzlrxwnnj3nv9d5nczqucvl32z6axd",
    "terra1ge6c28he6nfagzxma2k035nxy62akqrk3s535k",
    "terra1j78umayqmcd42n08mdv4hdw2n058ervcz4dvn8",
    "terra1vh2tkzxpc83fg0c2t65nd7k9psa0qvqxzr2uas",
    "terra10sqcnpyc3fsgrd94hmvlnhh45dqvhrxt5e94h5",
    "terra13fhx8j59l3age0s0jm5hkpcp3w8dxn8zz4jx6a",
  ];

  for (let contract of contracts) {
    console.log(`\n${chalk.yellow("Contract")}: ${chalk.cyan(contract)}`);
    const uri = uri_base.replace("<contract>", contract);
    console.log(`${chalk.yellow("URI")}: ${chalk.cyan(uri)}`);
    await updateURI(client, contract, uri);
  }
})();

async function updateURI(client: Client, contract: string, uri: string) {
  try {
    console.log(`\t Updating ${chalk.cyan(contract)} uri to ${uri}`);

    const tx = await performTransaction(client.terra, client.wallet, [
      new MsgExecuteContract(client.wallet.key.accAddress, contract, {
        update_config: {
          uri: uri,
        },
      }),
    ]);

    console.log(`\t\t Updated! ${chalk.green(`Tx: ${tx.txhash}`)}`);
  } catch (e: any) {
    console.log(chalk.red(`\t\t Failed to update, ${chalk.red(e?.rawLog || e)}`));
  }
}
