use std::ops::Add;
use spl_staking::{entrypoint::process_instruction};
use solana_program_test::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    system_program,
    pubkey::Pubkey,
    signature::{Signer, keypair::Keypair},
    transaction::Transaction,
};
use solana_program::program_pack::{Pack, IsInitialized};
use solana_program::system_instruction;
use solana_program::rent::Rent;
use solana_program::sysvar::rent;
use spl_token::state::{Account as TokenAccount, Mint};
use spl_staking::state::{ContractData, StakeType, UserData};


async fn get_user_data(pubkey: &Pubkey, banks_client: &mut BanksClient) -> UserData {
    let user_account = banks_client
        .get_account(pubkey.clone())
        .await
        .expect("get_account")
        .expect("user data account not found");
    UserData::unpack_from_slice(&user_account.data).unwrap()
}

async fn get_contract_data(pubkey: &Pubkey, banks_client: &mut BanksClient) -> ContractData {
    let contract_account = banks_client
        .get_account(pubkey.clone())
        .await
        .expect("get_account")
        .expect("contract pda data account not found");
    ContractData::unpack_from_slice(
        &contract_account.data
    ).unwrap()
}

async fn get_token_account_data(pubkey: &Pubkey, banks_client: & mut BanksClient) -> TokenAccount {
    let token_account = banks_client
        .get_account(pubkey.clone())
        .await
        .expect("get_account")
        .expect("token account not found");
    TokenAccount::unpack_from_slice(
        &token_account.data
    ).unwrap()
}


#[tokio::test]
async fn test_processor() {
    let program_id = Pubkey::new_unique();
    let token_mint = Keypair::new();

    let program_test = ProgramTest::new(
        "spl_staking",
        program_id,
        processor!(process_instruction),
    );

    // let mut context = program_test.start_with_context().await;
    // let mut banks_client = context.banks_client;
    // let payer = context.payer;
    // let recent_block_hash = context.last_blockhash;
    let (mut banks_client, payer, recent_block_hash) = program_test.start().await;
    //let x = program_test.start_with_context().await;
    let rent = Rent::default();
    let payer_pubkey = payer.pubkey();
    let mint_pubkey = token_mint.pubkey();
    let mint_decimals = 6_u64;
    let data_acct_pda_seeds: &[&[u8]] = &[b"spl_staking", &payer_pubkey.as_ref(), &mint_pubkey.as_ref()];
    let (data_acct_pda, _data_pda_bump) = Pubkey::find_program_address(
        data_acct_pda_seeds,
        &program_id
    );

    // ------------ Token mint Setup -----------
    let mint_txn = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer_pubkey,
                &mint_pubkey,
                rent.minimum_balance(Mint::LEN),
                Mint::LEN as u64,
                &spl_token::id()
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint_pubkey,
                &payer_pubkey,
                None,
                mint_decimals as u8
            ).unwrap()
        ],
        Some(&payer_pubkey),
        &[&payer, &token_mint],
        recent_block_hash
    );
    banks_client.process_transaction(mint_txn).await.unwrap();

    // --------------- Init contract test ----------------------
    // --------------- CASE 1 [SUCCESS] ------------------------
    let token_acct_keypair = Keypair::new();
    let minimum_stake_amount: u64 = 100 * 10u64.pow(mint_decimals as u32);
    let minimum_lock_duration: u64 = 100; // 100 seconds
    let normal_staking_apy: u64 = 100; // 10% per year
    let locked_staking_apy: u64 = 200; // 20% per year
    let early_withdrawal_fee: u64 = 50; // 5% per withdrawal
    let mut instruction_data = vec![0];
    instruction_data.extend(minimum_stake_amount.to_le_bytes().iter());
    instruction_data.extend(minimum_lock_duration.to_le_bytes().iter());
    instruction_data.extend(normal_staking_apy.to_le_bytes().iter());
    instruction_data.extend(locked_staking_apy.to_le_bytes().iter());
    instruction_data.extend(early_withdrawal_fee.to_le_bytes().iter());
    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer_pubkey,
                &token_acct_keypair.pubkey(),
                rent.minimum_balance(TokenAccount::LEN),
                TokenAccount::LEN as u64,
                &spl_token::id()
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &token_acct_keypair.pubkey(),
                &mint_pubkey,
                &payer_pubkey
            ).unwrap(),
            Instruction::new_with_bytes(
                program_id,
                &instruction_data,
                vec![
                    AccountMeta::new(payer.pubkey(), true),
                    AccountMeta::new(data_acct_pda, false),
                    AccountMeta::new(token_acct_keypair.pubkey(), false),
                    AccountMeta::new_readonly(token_mint.pubkey(), false),
                    AccountMeta::new_readonly(spl_token::id(), false),
                    AccountMeta::new_readonly(rent::ID, false),
                    AccountMeta::new_readonly(system_program::ID, false),
                ],
            )
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &token_acct_keypair], recent_block_hash);
    banks_client.process_transaction(transaction).await.unwrap();
    // Verify contract and token account states
    let contract_data = get_contract_data(&data_acct_pda, &mut banks_client).await;
    let contract_token_data = get_token_account_data(&token_acct_keypair.pubkey(), &mut banks_client).await;
    assert_eq!(
        contract_data.is_initialized,
        true
    );
    assert_eq!(
        contract_data.minimum_lock_duration,
        minimum_lock_duration
    );
    assert_eq!(
        contract_data.minimum_stake_amount,
        minimum_stake_amount
    );
    assert_eq!(
        contract_data.stake_token_account,
        token_acct_keypair.pubkey()
    );
    assert_eq!(
        contract_data.admin_pubkey,
        payer_pubkey
    );
    assert_eq!(
        contract_data.stake_token_mint,
        mint_pubkey
    );
    assert_eq!(
        contract_data.normal_staking_apy,
        normal_staking_apy
    );
    assert_eq!(
        contract_data.locked_staking_apy,
        locked_staking_apy
    );
    assert_eq!(
        contract_data.early_withdrawal_fee,
        early_withdrawal_fee
    );
    assert_eq!(
        contract_data.total_staked,
        0
    );
    assert_eq!(
        contract_data.total_earned,
        0
    );
    assert_eq!(
        contract_token_data.owner,
        data_acct_pda
    );
    assert_eq!(
        contract_token_data.amount,
        0
    );
    assert_eq!(
        contract_token_data.mint,
        mint_pubkey
    );
    assert_eq!(
        contract_token_data.is_initialized(),
        true
    );

    // --------------- Normal Staking Test ----------------------
    let user_token_account_keypair = Keypair::new();
    let (user_data_account_pubkey, _bump) = Pubkey::find_program_address(
        &[b"spl_staking_user", payer_pubkey.as_ref()],
        &program_id
    );
    let amount = 100*10u64.pow(mint_decimals as u32);
    let lock_duration: u64 = 0;
    let mut instruction_data = vec![1, 0];
    instruction_data.extend(amount.to_le_bytes().iter());
    instruction_data.extend(lock_duration.to_le_bytes().iter());
    // Set Up Claimer token account
    let user_token_acct_txn = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer_pubkey,
                &user_token_account_keypair.pubkey(),
                rent.minimum_balance(TokenAccount::LEN),
                TokenAccount::LEN as u64,
                &spl_token::id()
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &user_token_account_keypair.pubkey(),
                &mint_pubkey,
                &payer_pubkey
            ).unwrap(),
            spl_token::instruction::mint_to(
                &spl_token::id(),
                &mint_pubkey,
                &user_token_account_keypair.pubkey(),
                &payer_pubkey,
                &[],
                1000 * 10u64.pow(mint_decimals as u32)
            ).unwrap()
        ],
        Some(&payer_pubkey),
        &[&payer, &user_token_account_keypair],
        recent_block_hash
    );
    banks_client.process_transaction(user_token_acct_txn).await.unwrap();
    let mut stake_txn = Transaction::new_with_payer(
        &[
            Instruction::new_with_bytes(
                program_id,
                &instruction_data,
                vec![
                    AccountMeta::new(payer_pubkey, true),
                    AccountMeta::new(user_token_account_keypair.pubkey(), false),
                    AccountMeta::new(user_data_account_pubkey, false),
                    AccountMeta::new(token_acct_keypair.pubkey(), false),
                    AccountMeta::new(data_acct_pda, false),
                    AccountMeta::new_readonly(spl_token::ID, false),
                    AccountMeta::new_readonly(system_program::ID, false)
                ]
            )
        ],
        Some(&payer_pubkey)
    );
    stake_txn.sign(&[&payer], recent_block_hash);
    banks_client.process_transaction(stake_txn).await.unwrap();
    // Verify user data fields and token account balances
    let user_data = get_user_data(&user_data_account_pubkey, &mut banks_client).await;
    let contract_token_data = get_token_account_data(&token_acct_keypair.pubkey(), &mut banks_client).await;
    let contract_data = get_contract_data(&data_acct_pda, &mut banks_client).await;
    assert_eq!(user_data.is_initialized, true);
    assert_eq!(user_data.stake_type as u8, StakeType::NORMAL as u8);
    assert_eq!(user_data.lock_duration, lock_duration);
    assert_ne!(user_data.stake_ts, 0);
    assert_eq!(user_data.owner_pubkey, payer_pubkey);
    assert_eq!(contract_token_data.amount, amount);
    assert_eq!(contract_data.total_staked, amount);
    // --------------- Normal Re-staking Test ----------------------
    let re_stake_amount = 10*10u64.pow(mint_decimals as u32);
    let lock_duration: u64 = 0;
    let mut instruction_data = vec![1, 0];
    instruction_data.extend(re_stake_amount.to_le_bytes().iter());
    instruction_data.extend(lock_duration.to_le_bytes().iter());
    let mut re_stake_txn = Transaction::new_with_payer(
        &[
            Instruction::new_with_bytes(
                program_id,
                &instruction_data,
                vec![
                    AccountMeta::new(payer_pubkey, true),
                    AccountMeta::new(user_token_account_keypair.pubkey(), false),
                    AccountMeta::new(user_data_account_pubkey, false),
                    AccountMeta::new(token_acct_keypair.pubkey(), false),
                    AccountMeta::new(data_acct_pda, false),
                    AccountMeta::new_readonly(spl_token::ID, false),
                    AccountMeta::new_readonly(system_program::ID, false)
                ]
            )
        ],
        Some(&payer_pubkey)
    );
    re_stake_txn.sign(&[&payer], recent_block_hash);
    banks_client.process_transaction(re_stake_txn).await.unwrap();
    // Verify Side Effects
    let user_data = get_user_data(&user_data_account_pubkey, &mut banks_client).await;
    let contract_token_data = get_token_account_data(&token_acct_keypair.pubkey(), &mut banks_client).await;
    let contract_data = get_contract_data(&data_acct_pda, &mut banks_client).await;
    assert_eq!(user_data.total_staked, amount.add(re_stake_amount));
    assert_eq!(contract_data.total_staked, amount.add(re_stake_amount));
    assert_eq!(contract_token_data.amount, amount.add(re_stake_amount));

    // --------------- Locked Staking Tests -----------------

}