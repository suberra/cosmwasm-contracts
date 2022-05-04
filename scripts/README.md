# Deployment flow
Our scripts will attempt to deploy all suberra related contracts on the target network

1. Upload all artifacts & store code_id in `[chainId].json`
2. Instantiate the follow contracts (in order):
   1. Job Registry
   2. Subwallet Factory
       - Requires subwallet_code_id, anchor_market_contract & atoken_token
   3. Product Factory
       - Requires subscription_product_code_id
   4. P2P Contract
       - Requires job_registry_addr
   5. Token stream
3. Update config
   1. Add P2P contract to job registry
4. Create subwallet for deployer
   1. Skips if exist
5. Create subscription product
6. Subscribes

# Deployment

## Prerequisites
1. `yarn install`
2. `docker` installed
3. `ts-node` installed

## Setup
1. Building artifacts
   
    `./build_artifacts.sh`

2. Copy `env.template` file into `.env`

    **Env setting**
    ```shell
    WALLET="your deployer mnemonic"
    LCD_CLIENT_URL=https://bombay-lcd.terra.dev
    CHAIN_ID=bombay-12 # or localterra
    ```

3. Run deployment script
   
    `ts-node deploy.ts`

## Localterra deployment
    1. Empty `.env` file & `localterra.json`
    2. Run localterra with 0% tax_rate
        a. update genesis.json "tax_rate": "0.000000000000000000"
    4. Execute `ts-node deploy.ts`

