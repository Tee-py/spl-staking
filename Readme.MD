# SPL-STAKING

## Deployment Steps
1. run `chmod +x deploy.sh`
2. run `bash deploy.sh mainnet` for mainnet deployment and `bash deploy.sh devnet`
3. Copy the Program ID from the output

## Contract initialization
1. Run `cd scripts` and `npm install` 
2. Update the `PROGRAM_ID` and `TOKEN_DECIMALS` constants in `src/setup.ts` with the program id of the deployed program and the decimals of the token mint for the staking contract
3. Create a json file at `keys/${network}/admin.json` and put the wallet secret key of the admin as a string (e.g) `keys/mainnet/admin.json` with content `"wyefiwhbeuftvuyziftiuchyi"`.
4. Create another json file at `keys/${network}/mint_pub.json` and save the public key for the stake token mint (e.g) `keys/mainnet/mint_pub.json` with content `"AszrFF351DccvFWtrUMHbvcj2QeDChs7JChf8Rv29gKA"`
5. Edit the `setUp` function argument from lines `237-244` accordingly
    * `network`: `devnet` or `mainnet`
    * `minimumStakeAmount`: Minimum amount allowed for staking (no decimals)
    * `minimumLockDuration`: This is the minimum time in seconds for Locked staking (e.g) `24hrs = 24*60*60`
    * `normalStakingApy`: `APY for normal staking with decimals = 10 (i.e 1%/yr = 10, 50% = 500)
    * `lockedStakingApy`: APY for locked staking with decimals = 10
    * `earlyWithdrawalFee`: % charge for early withdrawals in locked staking with decimals = 10 (i.e 0.1% = 1)
    * `runInit`: set to `true` if you're initializing the contract
6. Run `npm run setup`