use solana_program::hash::Hash;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Signer, keypair::Keypair},
    transaction::Transaction,
};
use solana_program::program_pack::{Pack};
use solana_program::{system_instruction, system_program};
use solana_program::rent::Rent;
use solana_program::sysvar::rent;
use spl_token::state::{Account as TokenAccount, Mint};
use spl_staking::state::{ContractData, UserData};

pub async fn get_user_data(pubkey: &Pubkey, banks_client: &mut BanksClient) -> UserData {
    let user_account = banks_client
        .get_account(pubkey.clone())
        .await
        .expect("get_account")
        .expect("user data account not found");
    UserData::unpack_from_slice(&user_account.data).unwrap()
}

pub async fn get_contract_data(pubkey: &Pubkey, banks_client: &mut BanksClient) -> ContractData {
    let contract_account = banks_client
        .get_account(pubkey.clone())
        .await
        .expect("get_account")
        .expect("contract pda data account not found");
    ContractData::unpack_from_slice(
        &contract_account.data
    ).unwrap()
}

pub async fn get_token_account_data(pubkey: &Pubkey, banks_client: & mut BanksClient) -> TokenAccount {
    let token_account = banks_client
        .get_account(pubkey.clone())
        .await
        .expect("get_account")
        .expect("token account not found");
    TokenAccount::unpack_from_slice(
        &token_account.data
    ).unwrap()
}

pub async fn transfer_sol(
    payer: &Keypair,
    to_pubkey: Pubkey,
    amount: u64,
    banks_client: & mut BanksClient,
    recent_block_hash: Hash
) {
    let txn = Transaction::new_signed_with_payer(
        &[
            system_instruction::transfer(
                &payer.pubkey(),
                &to_pubkey,
                amount
            ),
        ],
        Some(&payer.pubkey()),
        &[payer],
        recent_block_hash
    );
    banks_client.process_transaction(txn).await.unwrap();
}

pub async fn set_up_mint(
    payer: &Keypair,
    mint: &Keypair,
    banks_client: & mut BanksClient,
    recent_block_hash: Hash,
    rent: Rent,
    mint_decimals: u64
) {
    let mint_txn = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &mint.pubkey(),
                rent.minimum_balance(Mint::LEN),
                Mint::LEN as u64,
                &spl_token::id()
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint.pubkey(),
                &payer.pubkey(),
                None,
                mint_decimals as u8
            ).unwrap()
        ],
        Some(&payer.pubkey()),
        &[payer, mint],
        recent_block_hash
    );
    banks_client.process_transaction(mint_txn).await.unwrap();
}

pub fn get_create_and_init_token_account_ix(
    payer_pubkey: Pubkey,
    acct_pubkey: Pubkey,
    rent: Rent,
    mint_pubkey: Pubkey
) -> (Instruction, Instruction) {
    (
        system_instruction::create_account(
            &payer_pubkey,
            &acct_pubkey,
            rent.minimum_balance(TokenAccount::LEN),
            TokenAccount::LEN as u64,
            &spl_token::id()
        ),
        spl_token::instruction::initialize_account(
            &spl_token::id(),
            &acct_pubkey,
            &mint_pubkey,
            &payer_pubkey
        ).unwrap()
    )
}

pub fn construct_init_txn(
    minimum_stake_amount: u64,
    minimum_lock_duration: u64,
    normal_staking_apy: u64,
    locked_staking_apy: u64,
    early_withdrawal_fee: u64,
    payer_pubkey: Pubkey,
    token_acct_pubkey: Pubkey,
    rent: Rent,
    mint_pubkey: Pubkey,
    program_id: Pubkey,
    data_acct_pda: Pubkey
) -> Transaction {
    let mut instruction_data = vec![0];
    instruction_data.extend(minimum_stake_amount.to_le_bytes().iter());
    instruction_data.extend(minimum_lock_duration.to_le_bytes().iter());
    instruction_data.extend(normal_staking_apy.to_le_bytes().iter());
    instruction_data.extend(locked_staking_apy.to_le_bytes().iter());
    instruction_data.extend(early_withdrawal_fee.to_le_bytes().iter());
    let (create_ix, init_ix) = get_create_and_init_token_account_ix(
        payer_pubkey.clone(),
        token_acct_pubkey.clone(),
        rent.clone(),
        mint_pubkey.clone()
    );
    Transaction::new_with_payer(
        &[
            create_ix,
            init_ix,
            Instruction::new_with_bytes(
                program_id,
                &instruction_data,
                vec![
                    AccountMeta::new(payer_pubkey, true),
                    AccountMeta::new(data_acct_pda, false),
                    AccountMeta::new(token_acct_pubkey, false),
                    AccountMeta::new_readonly(mint_pubkey, false),
                    AccountMeta::new_readonly(spl_token::id(), false),
                    AccountMeta::new_readonly(rent::ID, false),
                    AccountMeta::new_readonly(system_program::ID, false),
                ],
            )
        ],
        Some(&payer_pubkey),
    )
}

pub async fn set_up_token_account(
    payer: &Keypair,
    token_account_keypair: &Keypair,
    owner: Option<Pubkey>,
    rent: Rent,
    mint_pubkey: Pubkey,
    mint_amount: u64,
    banks_client: & mut BanksClient,
    recent_block_hash: Hash
){
    let txn = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &token_account_keypair.pubkey(),
                rent.minimum_balance(TokenAccount::LEN),
                TokenAccount::LEN as u64,
                &spl_token::id()
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &token_account_keypair.pubkey(),
                &mint_pubkey,
                &payer.pubkey()
            ).unwrap(),
            spl_token::instruction::mint_to(
                &spl_token::id(),
                &mint_pubkey,
                &token_account_keypair.pubkey(),
                &payer.pubkey(),
                &[],
                mint_amount
            ).unwrap()
        ],
        Some(&payer.pubkey()),
        &[&payer, &token_account_keypair],
        recent_block_hash
    );
    banks_client.process_transaction(txn).await.unwrap();
    match owner {
        Some(pk) => {
            let change_owner_ix = spl_token::instruction::set_authority(
                &spl_token::id(),
                &token_account_keypair.pubkey(),
                Some(&pk),
                spl_token::instruction::AuthorityType::AccountOwner,
                &payer.pubkey(),
                &[&payer.pubkey()]
            ).unwrap();
            let txn = Transaction::new_signed_with_payer(
                &[
                    change_owner_ix
                ],
                Some(&payer.pubkey()),
                &[&payer],
                recent_block_hash
            );
            banks_client.process_transaction(txn).await.unwrap();
        },
        None => {}
    };
}

pub async fn perform_stake(
    program_id: Pubkey,
    payer: &Keypair,
    user_tkn_acct_pk: Pubkey,
    contract_tkn_acct_pk: Pubkey,
    user_data_acct_pk: Pubkey,
    contract_data_acct_pk: Pubkey,
    stake_type: u8,
    amount: u64,
    lock_duration: u64,
    banks_client: & mut BanksClient,
    recent_block_hash: Hash
) {
    let mut instruction_data = vec![1, stake_type];
    instruction_data.extend(amount.to_le_bytes().iter());
    instruction_data.extend(lock_duration.to_le_bytes().iter());
    let mut stake_txn = Transaction::new_with_payer(
        &[
            Instruction::new_with_bytes(
                program_id,
                &instruction_data,
                vec![
                    AccountMeta::new(payer.pubkey(), true),
                    AccountMeta::new(user_tkn_acct_pk, false),
                    AccountMeta::new(user_data_acct_pk, false),
                    AccountMeta::new(contract_tkn_acct_pk, false),
                    AccountMeta::new(contract_data_acct_pk, false),
                    AccountMeta::new_readonly(spl_token::ID, false),
                    AccountMeta::new_readonly(system_program::ID, false)
                ]
            )
        ],
        Some(&payer.pubkey())
    );
    stake_txn.sign(&[&payer], recent_block_hash);
    banks_client.process_transaction(stake_txn).await.unwrap();
}

pub async fn perform_unstake(
    program_id: Pubkey,
    payer: &Keypair,
    user_tkn_acct_pk: Pubkey,
    contract_tkn_acct_pk: Pubkey,
    user_data_acct_pk: Pubkey,
    contract_data_acct_pk: Pubkey,
    banks_client: & mut BanksClient,
    recent_block_hash: Hash
) {
    let instruction_data = vec![2];
    let mut unstake_txn = Transaction::new_with_payer(
        &[
            Instruction::new_with_bytes(
                program_id,
                &instruction_data,
                vec![
                    AccountMeta::new(payer.pubkey(), true),
                    AccountMeta::new(user_tkn_acct_pk, false),
                    AccountMeta::new(user_data_acct_pk, false),
                    AccountMeta::new(contract_tkn_acct_pk, false),
                    AccountMeta::new(contract_data_acct_pk, false),
                    AccountMeta::new_readonly(spl_token::ID, false)
                ]
            )
        ],
        Some(&payer.pubkey())
    );
    unstake_txn.sign(&[&payer], recent_block_hash);
    banks_client.process_transaction(unstake_txn).await.unwrap();
}