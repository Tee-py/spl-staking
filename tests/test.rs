mod utils;

use utils::{set_up_mint, get_user_data, get_contract_data, get_token_account_data};
use std::ops::Add;
use solana_program::native_token::LAMPORTS_PER_SOL;
use spl_staking::{entrypoint::process_instruction};
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Signer, keypair::Keypair},
};
use solana_program::program_pack::{IsInitialized};
use solana_program::rent::Rent;
use spl_staking::state::{StakeType};
use crate::utils::{
    construct_init_txn,
    perform_change_transfer_config,
    perform_stake,
    perform_unstake,
    set_up_token_account,
    transfer_sol
};

#[tokio::test]
async fn test_processor() {
    let program_id = Pubkey::new_unique();
    let token_mint = Keypair::new();

    let program_test = ProgramTest::new(
        "spl_staking",
        program_id,
        processor!(process_instruction),
    );

    let (mut banks_client, payer, recent_block_hash) = program_test.start().await;
    let rent = Rent::default();
    let payer_pubkey = payer.pubkey();
    let mint_pubkey = token_mint.pubkey();
    let mint_decimals = 9_u64;
    let fee_basis_point: u64 = 800;
    let max_fee: u64 = 9536743164 * 10u64.pow(mint_decimals as u32);
    let data_acct_pda_seeds: &[&[u8]] = &[b"spl_staking", &payer_pubkey.as_ref(), &mint_pubkey.as_ref()];
    let (data_acct_pda, _data_pda_bump) = Pubkey::find_program_address(
        data_acct_pda_seeds,
        &program_id
    );

    // ------------ Token mint Setup -----------
    set_up_mint(
        &payer,
        &token_mint,
        &mut banks_client,
        recent_block_hash,
        rent.clone(),
        mint_decimals,
        fee_basis_point,
        max_fee
    ).await;
    // --------------- Init contract test ----------------------
    // --------------- CASE 1 [SUCCESS] ------------------------
    let token_acct_keypair = Keypair::new();
    let minimum_stake_amount: u64 = 100 * 10u64.pow(mint_decimals as u32);
    let mint_amount: u64 = 10000 * 10u64.pow(mint_decimals as u32);
    let minimum_lock_duration: u64 = 100; // 100 seconds
    let normal_staking_apy: u64 = 26390; // 2639% per year
    let locked_staking_apy: u64 = 60570; // 6057% per year
    let early_withdrawal_fee: u64 = 100; // 5% per withdrawal
    let mut transaction = construct_init_txn(
        minimum_stake_amount,
        minimum_lock_duration,
        normal_staking_apy,
        locked_staking_apy,
        mint_amount,
        early_withdrawal_fee,
        fee_basis_point,
        max_fee,
        payer_pubkey,
        token_acct_keypair.pubkey(),
        rent,
        mint_pubkey,
        program_id,
        data_acct_pda
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
        contract_data.fee_basis_points,
        fee_basis_point
    );
    assert_eq!(
        contract_data.max_fee,
        max_fee
    );
    assert_eq!(
        contract_token_data.owner,
        data_acct_pda
    );
    assert_eq!(
        contract_token_data.amount,
        mint_amount
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
    let amount = 10000*10u64.pow(mint_decimals as u32);
    let lock_duration: u64 = 0;

    // Set Up Claimer token account
    let mint_amount = 15000 * 10u64.pow(mint_decimals as u32);
    set_up_token_account(
        &payer,
        &user_token_account_keypair,
        None,
        rent.clone(),
        mint_pubkey.clone(),
        mint_amount,
        &mut banks_client,
        recent_block_hash
    ).await;
    // perform normal stake
    perform_stake(
        program_id.clone(),
        &payer,
        user_token_account_keypair.pubkey(),
        token_acct_keypair.pubkey(),
        user_data_account_pubkey.clone(),
        data_acct_pda.clone(),
        mint_pubkey.clone(),
        StakeType::NORMAL as u8,
        amount,
        mint_decimals,
        lock_duration,
        &mut banks_client,
        recent_block_hash
    ).await;
    // Verify user data fields and token account balances
    let user_data = get_user_data(&user_data_account_pubkey, &mut banks_client).await.unwrap();
    println!("{}", user_data.is_initialized);
    let contract_data = get_contract_data(&data_acct_pda, &mut banks_client).await;
    assert_eq!(user_data.is_initialized, true);
    assert_eq!(user_data.stake_type as u8, StakeType::NORMAL as u8);
    assert_eq!(user_data.lock_duration, lock_duration);
    assert_ne!(user_data.stake_ts, 0);
    assert_eq!(user_data.owner_pubkey, payer_pubkey);
    assert_eq!(user_data.total_staked, amount);
    assert_eq!(contract_data.total_staked, amount);
    // --------------- Normal Re-staking Test ----------------------
    let re_stake_amount = 100*10u64.pow(mint_decimals as u32);
    let lock_duration: u64 = 0;
    perform_stake(
        program_id.clone(),
        &payer,
        user_token_account_keypair.pubkey(),
        token_acct_keypair.pubkey(),
        user_data_account_pubkey.clone(),
        data_acct_pda.clone(),
        mint_pubkey.clone(),
        StakeType::NORMAL as u8,
        re_stake_amount,
        mint_decimals,
        lock_duration,
        &mut banks_client,
        recent_block_hash
    ).await;
    // Verify Side Effects
    let user_data = get_user_data(&user_data_account_pubkey, &mut banks_client).await.unwrap();
    let contract_data = get_contract_data(&data_acct_pda, &mut banks_client).await;
    assert_eq!(user_data.total_staked, amount.add(re_stake_amount));
    assert_eq!(contract_data.total_staked, amount.add(re_stake_amount));
    // ---------- Normal Un-staking Tests -------------
    // perform_unstake(
    //     program_id.clone(),
    //     &payer,
    //     user_token_account_keypair.pubkey(),
    //     token_acct_keypair.pubkey(),
    //     user_data_account_pubkey.clone(),
    //     data_acct_pda.clone(),
    //     mint_pubkey.clone(),
    //     &mut banks_client,
    //     recent_block_hash,
    //     mint_decimals
    // ).await;
    // let user_token_data = get_token_account_data(&user_token_account_keypair.pubkey(), &mut banks_client).await;
    // let contract_data = get_contract_data(&data_acct_pda, &mut banks_client).await;
    // let user_data = get_user_data(&user_data_account_pubkey, &mut banks_client).await;
    // assert_eq!(user_data.total_staked, 0);
    // assert_eq!(contract_data.total_staked, 0);
    // assert_eq!(user_data.interest_accrued, 0);
    // assert_eq!(user_token_data.amount, mint_amount.add(fee));

    // --------------- Locked Staking Tests -----------------
    let new_payer = Keypair::new();
    let (new_payer_data_acct_pk, _bump) = Pubkey::find_program_address(
        &[b"spl_staking_user", new_payer.pubkey().as_ref()],
        &program_id
    );
    let payer_token_account_keypair = Keypair::new();
    let mint_amount = 20000 * 10u64.pow(mint_decimals as u32);
    let stake_amount = 10000*10u64.pow(mint_decimals as u32);
    let lock_duration = 24*60*60;
    transfer_sol(
        &payer,
        new_payer.pubkey().clone(),
        10*LAMPORTS_PER_SOL,
        &mut banks_client,
        recent_block_hash
    ).await;
    set_up_token_account(
        &payer,
        &payer_token_account_keypair,
        Some(new_payer.pubkey().clone()),
        rent.clone(),
        mint_pubkey.clone(),
        mint_amount,
        &mut banks_client,
        recent_block_hash
    ).await;
    perform_stake(
        program_id.clone(),
        &new_payer,
        payer_token_account_keypair.pubkey(),
        token_acct_keypair.pubkey(),
        new_payer_data_acct_pk.clone(),
        data_acct_pda.clone(),
        mint_pubkey.clone(),
        StakeType::LOCKED as u8,
        stake_amount,
        mint_decimals,
        lock_duration,
        &mut banks_client,
        recent_block_hash
    ).await;
    let expected_total_staked = amount.add(re_stake_amount).add(stake_amount);
    let user_data = get_user_data(&new_payer_data_acct_pk, &mut banks_client).await.unwrap();
    let contract_data = get_contract_data(&data_acct_pda, &mut banks_client).await;
    assert_eq!(user_data.total_staked, stake_amount);
    assert_eq!(user_data.stake_type as u8, StakeType::LOCKED as u8);
    assert_eq!(user_data.is_initialized, true);
    assert_eq!(user_data.lock_duration, lock_duration);
    assert_ne!(user_data.stake_ts, 0);
    assert_eq!(user_data.owner_pubkey, new_payer.pubkey());
    assert_eq!(contract_data.total_staked, expected_total_staked);
    // ----------- Locked Re-staking Test --------------------
    let re_stake_amount = 100*10u64.pow(mint_decimals as u32);
    let new_lock_duration = 2*24*60*60;
    let _initial_user_data = get_user_data(&new_payer_data_acct_pk, &mut banks_client).await.unwrap();
    perform_stake(
        program_id.clone(),
        &new_payer,
        payer_token_account_keypair.pubkey(),
        token_acct_keypair.pubkey(),
        new_payer_data_acct_pk.clone(),
        data_acct_pda.clone(),
        mint_pubkey.clone(),
        StakeType::LOCKED as u8,
        re_stake_amount,
        mint_decimals,
        new_lock_duration,
        &mut banks_client,
        recent_block_hash
    ).await;
    let expected_total_staked = expected_total_staked.add(re_stake_amount);
    let expected_user_total_staked = user_data.total_staked.add(re_stake_amount);
    let final_user_data = get_user_data(&new_payer_data_acct_pk, &mut banks_client).await.unwrap();
    let contract_data = get_contract_data(&data_acct_pda, &mut banks_client).await;
    assert_eq!(final_user_data.lock_duration, new_lock_duration);
    assert_eq!(final_user_data.total_staked, expected_user_total_staked);
    assert_eq!(contract_data.total_staked, expected_total_staked);
    // ---------- Locked Un-staking Tests -------------
    perform_unstake(
        program_id.clone(),
        &new_payer,
        payer_token_account_keypair.pubkey(),
        token_acct_keypair.pubkey(),
        new_payer_data_acct_pk.clone(),
        data_acct_pda.clone(),
        mint_pubkey.clone(),
        &mut banks_client,
        recent_block_hash,
        mint_decimals
    ).await;
    let user_data = get_user_data(&new_payer_data_acct_pk, &mut banks_client).await;
    let after_unstake_bal = get_token_account_data(
        &payer_token_account_keypair.pubkey(),
        &mut banks_client
    ).await;
    match user_data {
        Ok(_data) => assert!(false),
        Err(_e) => assert!(true)
    };
    let expected_unstake_amt = expected_user_total_staked - (expected_user_total_staked * 10)/100;
    let expected_unstake_amt_with_fee = expected_unstake_amt + (expected_unstake_amt * 12)/100;
    let actual_unstake_amt = expected_unstake_amt_with_fee - (expected_unstake_amt_with_fee * fee_basis_point)/10000;
    assert_eq!(mint_amount - expected_user_total_staked + actual_unstake_amt, after_unstake_bal.amount);

    // Stake After Un-staking
    let stake_amount = 100*10u64.pow(mint_decimals as u32);
    let lock_duration = 50*60*60;
    perform_stake(
        program_id.clone(),
        &new_payer,
        payer_token_account_keypair.pubkey(),
        token_acct_keypair.pubkey(),
        new_payer_data_acct_pk.clone(),
        data_acct_pda.clone(),
        mint_pubkey.clone(),
        StakeType::LOCKED as u8,
        stake_amount,
        mint_decimals,
        lock_duration,
        &mut banks_client,
        recent_block_hash
    ).await;
    let user_data = get_user_data(&new_payer_data_acct_pk, &mut banks_client).await.unwrap();
    assert_eq!(user_data.total_staked, stake_amount);
    assert_eq!(user_data.is_initialized, true);
    assert_eq!(user_data.stake_type as u8, StakeType::LOCKED as u8);
    assert_eq!(user_data.interest_accrued, 0);
    assert_eq!(user_data.owner_pubkey, new_payer.pubkey());
    assert_eq!(user_data.lock_duration, lock_duration);
    // ------------- Change Transfer Config Test ----------------
    let fee_basis_points = 1000;
    let max_fee = 1000 * 10u64.pow(mint_decimals as u32);
    perform_change_transfer_config(
        program_id,
        &payer,
        data_acct_pda.clone(),
        fee_basis_points,
        max_fee,
        &mut banks_client,
        recent_block_hash
    ).await;
    let contract_data = get_contract_data(&data_acct_pda, &mut banks_client).await;
    assert_eq!(contract_data.fee_basis_points, fee_basis_points);
    assert_eq!(contract_data.max_fee, max_fee)
}