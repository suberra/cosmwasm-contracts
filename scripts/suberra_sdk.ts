import { Coin, LCDClient, MsgExecuteContract } from "@terra-money/terra.js";
import { toEncodedBinary } from "./helpers";

export async function getSubwallet(lcd: LCDClient, subwallet_factory: string, terraAddress: string): Promise<string | null> {
    if (!terraAddress) {
        throw new Error("terraAddress cannot be empty");
    }
    return lcd.wasm.contractQuery<string>(
        subwallet_factory,
        {
            get_subwallet_address: {
                owner_address: terraAddress,
            },
        }
    );
}

export async function getCreatedSubscriptions(lcd: LCDClient, product_factory: string, terraAddress: string): Promise<string[] | null> {
    if (!terraAddress) {
        throw new Error("terraAddress cannot be empty");
    }
    return lcd.wasm.contractQuery<{ products: string[] }>(
        product_factory,
        {
            products_by_owner: {
                owner: terraAddress,
            },
        }
    ).then(res => res.products);
}


export function createSubscribeMsgs(walletAddress: string, subwallet: string, subscription_contract: string) {
    // Give a high allowance
    const allowanceAmount = new Coin("uusd", Number.MAX_SAFE_INTEGER);

    const increaseAllowance = new MsgExecuteContract(walletAddress, subwallet, {
        increase_allowance: {
            spender: subscription_contract,
            amount: allowanceAmount.toData(),
        },
    });

    const subscribeTo = new MsgExecuteContract(walletAddress, subwallet, {
        execute: {
            msgs: [
                {
                    wasm: {
                        execute: {
                            funds: [],
                            contract_addr: subscription_contract,
                            msg: toEncodedBinary({
                                subscribe: {},
                            }),
                        },
                    },
                },
            ],
        },
    });

    return [increaseAllowance, subscribeTo];
}

export async function getIsSubscribed(lcd: LCDClient, subscription_contract: string, subwallet: string) {
    return lcd.wasm
        .contractQuery<{ is_active: boolean }>(subscription_contract, {
            subscription: {
                subscriber: subwallet,
            },
        }).then(res => res.is_active).catch(() => false);
}
