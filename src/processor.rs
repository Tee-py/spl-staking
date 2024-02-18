use std::ops::Add;
use solana_program::{
    account_info::{AccountInfo, next_account_info},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_pack::{Pack},
    pubkey::Pubkey,
    system_instruction,
    sysvar::{Sysvar},
    program::{invoke_signed, invoke},
    //clock::Clock,
    msg,
};
use solana_program::clock::Clock;
//use solana_program::instruction::AccountMeta;
use solana_program::rent::Rent;
use spl_token::state::{Account as TokenAccount};
use crate::instruction::Instruction as ContractInstruction;
use crate::state::{ContractData, StakeType, UserData};


pub struct Processor;

impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8]
    ) -> ProgramResult {
        let instruction = ContractInstruction::unpack(instruction_data)?;
        match instruction {
            ContractInstruction::Init {
                minimum_stake_amount, minimum_lock_duration,
                normal_staking_apy, locked_staking_apy,
                early_withdrawal_fee, tax_percent
            } => {
                msg!("Staking [Info]: Init contract instruction");
                Self::init(
                    program_id, accounts,
                    minimum_stake_amount, minimum_lock_duration,
                    normal_staking_apy, locked_staking_apy,
                    early_withdrawal_fee, tax_percent
                )
            },
            ContractInstruction::Stake {
                stake_type, amount,
                lock_duration
            } => {
                msg!("Staking [Info]: Stake Instruction");
                Self::stake(
                    program_id,
                    accounts,
                    stake_type,
                    amount,
                    lock_duration
                )
            },
            ContractInstruction::UnStake => {
                msg!("Staking [Info]: Unstake Instruction");
                Self::unstake(
                    program_id,
                    accounts
                )
            }
        }
    }

    fn init(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        minimum_stake_amount: u64,
        minimum_lock_duration: u64,
        normal_staking_apy: u64,
        locked_staking_apy: u64,
        early_withdrawal_fee: u64,
        token_tax_percent: u64
    ) -> ProgramResult {
        // Get all accounts sent to the instruction
        let accounts_info_iter = &mut accounts.iter();
        let admin = next_account_info(accounts_info_iter)?;
        let data_account = next_account_info(accounts_info_iter)?;
        let token_account = next_account_info(accounts_info_iter)?;
        let mint_info = next_account_info(accounts_info_iter)?;
        let token_program_info = next_account_info(accounts_info_iter)?;
        let system_program_account = next_account_info(accounts_info_iter)?;

        // perform necessary checks
        if !admin.is_signer {
            return Err(ProgramError::MissingRequiredSignature.into());
        }
        if !data_account.is_writable {
            return Err(ProgramError::InvalidAccountData.into());
        }
        if minimum_stake_amount == 0 {
            msg!("Staking [Error]: Cannot init contract with zero minimum stake amount");
            return Err(ProgramError::InvalidInstructionData.into());
        }
        if token_program_info.key == spl_token_2022::ID && token_tax_percent < 1 {
            msg!("Staking [Error]: Instruction specified TOKEN_2022 but invalid tax percentage");
            return Err(ProgramError::InvalidInstructionData.into())
        }

        // Create Contract Data account with the PDA
        let seeds: &[&[u8]] = &[
            b"spl_staking",
            admin.key.as_ref(),
            mint_info.key.as_ref()
        ];
        let (pda_addr, pda_bump) = Pubkey::find_program_address(seeds, program_id);
        if &pda_addr != data_account.key {
            msg!("PDA Addr Account Mismatch");
            return Err(ProgramError::InvalidAccountData.into());
        };
        let rent = &Rent::get()?;
        let required_lamports = rent
            .minimum_balance(ContractData::LEN)
            .max(1)
            .saturating_sub(data_account.lamports());
        let contract_seeds: &[&[u8]] = &[b"spl_staking", admin.key.as_ref(), mint_info.key.as_ref(), &[pda_bump]];
        invoke_signed(
            &system_instruction::create_account(
                admin.key,
                data_account.key,
                required_lamports,
                ContractData::LEN as u64,
                program_id,
            ),
            &[
                admin.clone(),
                data_account.clone(),
                mint_info.clone(),
                system_program_account.clone(),
            ],
            &[contract_seeds],
        )?;

        // Change ownership of the token account
        let change_owner_ix = spl_token::instruction::set_authority(
            &spl_token::id(),
            token_account.key,
            Some(&pda_addr),
            spl_token::instruction::AuthorityType::AccountOwner,
            admin.key,
            &[&admin.key]
        )?;
        invoke(
            &change_owner_ix,
            &[
                token_account.clone(),
                admin.clone(),
                token_program_info.clone(),
            ],
        )?;

        // Update contract data
        let mut contract_data = ContractData::unpack_unchecked(&data_account.data.borrow())?;
        if contract_data.is_initialized {
            return Err(ProgramError::AccountAlreadyInitialized.into())
        }
        contract_data.is_initialized = true;
        contract_data.admin_pubkey = *admin.key;
        contract_data.stake_token_mint = *mint_info.key;
        contract_data.minimum_stake_amount = minimum_stake_amount;
        contract_data.minimum_lock_duration = minimum_lock_duration;
        contract_data.stake_token_account = *token_account.key;
        contract_data.normal_staking_apy = normal_staking_apy;
        contract_data.locked_staking_apy = locked_staking_apy;
        contract_data.early_withdrawal_fee = early_withdrawal_fee;
        contract_data.total_earned = 0;
        contract_data.total_staked = 0;

        ContractData::pack(contract_data, &mut data_account.try_borrow_mut_data()?)?;
        Ok(())
    }

    fn stake(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        stake_type: StakeType,
        amount: u64,
        lock_duration: u64
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let user_info = next_account_info(account_info_iter)?;
        let user_token_account_info = next_account_info(account_info_iter)?;
        let user_data_account_info = next_account_info(account_info_iter)?;
        let contract_token_account_info = next_account_info(account_info_iter)?;
        let contract_data_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        let contract_data = ContractData::unpack_from_slice(&contract_data_account_info.data.borrow())?;
        let user_token_account_data = TokenAccount::unpack_from_slice(&user_token_account_info.data.borrow())?;
        let contract_token_account_data = TokenAccount::unpack_from_slice(&contract_token_account_info.data.borrow())?;


        if !user_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature.into())
        }
        // Verify user and contract token accounts
        if user_token_account_data.owner != *user_info.key {
            msg!("Staking [Error]: Invalid user token account");
            return Err(ProgramError::InvalidAccountData.into())
        }
        if user_token_account_data.mint != contract_data.stake_token_mint {
            msg!("Staking [Error]: Invalid user token account mint");
            return Err(ProgramError::InvalidAccountData.into())
        }
        if user_token_account_data.amount < contract_data.minimum_stake_amount {
            msg!("Staking [Error]: Insufficient user token balance for staking");
            return Err(ProgramError::InsufficientFunds.into())
        }

        // verify the contract data pda
        let (contract_data_pda, _c_bump) = Pubkey::find_program_address(
            &[b"spl_staking", contract_data.admin_pubkey.as_ref(), contract_data.stake_token_mint.as_ref()],
            program_id
        );
        if &contract_data_pda != contract_data_account_info.key {
            msg!("Staking [Error]: Invalid contract data account");
            return Err(ProgramError::InvalidAccountData.into())
        }
        if contract_token_account_info.key != &contract_data.stake_token_account {
            msg!("Staking [Error]: Invalid contract token account");
            return Err(ProgramError::InvalidAccountData.into())
        }
        if contract_data.stake_token_mint != contract_token_account_data.mint {
            msg!("Staking [Error]: Invalid contract token account mint");
            return Err(ProgramError::InvalidAccountData.into())
        }
        if contract_data_pda != contract_token_account_data.owner {
            msg!("Staking [Error]: Invalid contract token account owner");
            return Err(ProgramError::InvalidAccountData.into())
        }
        match stake_type {
            StakeType::NORMAL => {
                msg!("Staking [Info]: Performing Normal Staking");
                Self::perform_staking(
                    program_id,
                    user_info,
                    user_token_account_info,
                    user_data_account_info,
                    system_program_info,
                    token_program_info,
                    contract_token_account_info,
                    contract_data_account_info,
                    StakeType::NORMAL,
                    amount,
                    contract_data.normal_staking_apy,
                    0
                )
            },
            StakeType::LOCKED => {
                msg!("Staking [Info]: Locked Staking");
                if lock_duration < contract_data.minimum_lock_duration {
                    msg!("Staking [Error]: Lock duration is less than minimum lock duration❌");
                    return Err(ProgramError::InvalidInstructionData.into())
                }
                Self::perform_staking(
                    program_id,
                    user_info,
                    user_token_account_info,
                    user_data_account_info,
                    system_program_info,
                    token_program_info,
                    contract_token_account_info,
                    contract_data_account_info,
                    StakeType::LOCKED,
                    amount,
                    contract_data.locked_staking_apy,
                    lock_duration
                )
            }
        }
    }

    fn unstake(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let user_info = next_account_info(account_info_iter)?;
        let user_token_account_info = next_account_info(account_info_iter)?;
        let user_data_account_info = next_account_info(account_info_iter)?;
        let contract_token_account_info = next_account_info(account_info_iter)?;
        let contract_data_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let contract_data = ContractData::unpack_from_slice(&contract_data_account_info.data.borrow())?;
        let user_data = UserData::unpack_from_slice(&user_data_account_info.data.borrow())?;
        let user_token_account_data = TokenAccount::unpack_from_slice(&user_token_account_info.data.borrow())?;
        let contract_token_account_data = TokenAccount::unpack_from_slice(&contract_token_account_info.data.borrow())?;
        if !user_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature.into())
        }
        // Verify user and contract token accounts
        if user_token_account_data.owner != *user_info.key {
            msg!("Staking [Error]: Invalid user token account");
            return Err(ProgramError::InvalidAccountData.into())
        }
        if user_token_account_data.mint != contract_data.stake_token_mint {
            msg!("Staking [Error]: Invalid user token account mint");
            return Err(ProgramError::InvalidAccountData.into())
        }
        if user_token_account_data.amount < contract_data.minimum_stake_amount {
            msg!("Staking [Error]: Insufficient user token balance for staking");
            return Err(ProgramError::InsufficientFunds.into())
        }

        // verify the contract data pda
        let (contract_data_pda, _c_bump) = Pubkey::find_program_address(
            &[b"spl_staking", contract_data.admin_pubkey.as_ref(), contract_data.stake_token_mint.as_ref()],
            program_id
        );
        if &contract_data_pda != contract_data_account_info.key {
            msg!("Staking [Error]: Invalid contract data account");
            return Err(ProgramError::InvalidAccountData.into())
        }
        if contract_token_account_info.key != &contract_data.stake_token_account {
            msg!("Staking [Error]: Invalid contract token account");
            return Err(ProgramError::InvalidAccountData.into())
        }
        if contract_data.stake_token_mint != contract_token_account_data.mint {
            msg!("Staking [Error]: Invalid contract token account mint");
            return Err(ProgramError::InvalidAccountData.into())
        }
        if contract_data_pda != contract_token_account_data.owner {
            msg!("Staking [Error]: Invalid contract token account owner");
            return Err(ProgramError::InvalidAccountData.into())
        };
        match user_data.stake_type {
            StakeType::NORMAL => {
                msg!("Staking [Info]: Performing Normal Un-staking");
                Self::perform_unstake(
                    program_id,
                    user_info,
                    user_token_account_info,
                    user_data_account_info,
                    token_program_info,
                    contract_token_account_info,
                    contract_data_account_info,
                    StakeType::NORMAL,
                    contract_data.normal_staking_apy,
                )
            },
            StakeType::LOCKED => {
                msg!("Staking [Info]: Locked Un-staking");
                Self::perform_unstake(
                    program_id,
                    user_info,
                    user_token_account_info,
                    user_data_account_info,
                    token_program_info,
                    contract_token_account_info,
                    contract_data_account_info,
                    StakeType::LOCKED,
                    contract_data.locked_staking_apy
                )
            }
        }
    }

    fn perform_unstake<'a>(
        program_id: &Pubkey,
        user_info: &AccountInfo<'a>,
        user_token_account_info: &AccountInfo<'a>,
        user_data_account: &AccountInfo<'a>,
        token_program_info: &AccountInfo<'a>,
        contract_token_account_info: &AccountInfo<'a>,
        contract_data_account: &AccountInfo<'a>,
        stake_type: StakeType,
        apy: u64,
    ) -> ProgramResult {
        // verify the user data account
        let seeds: &[&[u8]] = &[b"spl_staking_user", user_info.key.as_ref()];
        let (ns_user_data_pda, _bump) = Pubkey::find_program_address(
            seeds,
            program_id
        );
        if *user_data_account.key != ns_user_data_pda {
            msg!("Staking [Error]: User data account and generated pda mismatch");
            return Err(ProgramError::InvalidAccountData.into())
        }

        let clock = Clock::get()?;
        let current_ts = clock.unix_timestamp as u64;
        let mut contract_data = ContractData::unpack_unchecked(&contract_data_account.data.borrow())?;
        let mut user_data = UserData::unpack_from_slice(
            &user_data_account.data.borrow()
        )?;

        let  amount_out = match stake_type {
            StakeType::NORMAL => {
                let stake_duration = current_ts - user_data.stake_ts;
                if stake_duration < 86400 {
                    msg!("Staking [Info]: Cannot Unstake before 24 hrs");
                    return Err(ProgramError::InvalidAccountData.into());
                }
                let mut interest_accrued = (apy * user_data.total_staked * stake_duration)/31536000000;
                contract_data.total_earned = contract_data.total_earned.saturating_add(interest_accrued);
                interest_accrued = interest_accrued.add(user_data.interest_accrued);
                msg!("Staking[Info]: Interest Accrued: {}\nStake Duration: {}", interest_accrued, stake_duration);
                let amount_out = user_data.total_staked.add(interest_accrued);
                amount_out
            },
            StakeType::LOCKED => {
                let stake_duration = current_ts - user_data.stake_ts;
                let amount_out: u64;
                if stake_duration >= user_data.lock_duration {
                    let mut interest_accrued = (apy * user_data.total_staked * stake_duration)/31536000000;
                    contract_data.total_earned = contract_data.total_earned.saturating_add(interest_accrued);
                    interest_accrued = interest_accrued.add(user_data.interest_accrued);
                    amount_out = interest_accrued.add(user_data.total_staked);
                } else {
                    let early_unstake_charge = (contract_data.early_withdrawal_fee * user_data.total_staked)/1000;
                    amount_out = user_data.total_staked.saturating_sub(early_unstake_charge);
                }
                msg!("Staking [Info]: Amount Out: {} Total Staked: {}", amount_out, user_data.total_staked);
                amount_out
            }
        };
        // Transfer tokens to the user
        let seeds: &[&[u8]] = &[
            b"spl_staking",
            contract_data.admin_pubkey.as_ref(),
            contract_data.stake_token_mint.as_ref()
        ];
        let (authority_pda, pda_bump) = Pubkey::find_program_address(seeds, program_id);
        let token_transfer_ix = spl_token::instruction::transfer(
            token_program_info.key,
            contract_token_account_info.key,
            user_token_account_info.key,
            &authority_pda,
            &[&authority_pda],
            amount_out
        )?;
        let signer_seeds: &[&[u8]] = &[
            b"spl_staking",
            contract_data.admin_pubkey.as_ref(),
            contract_data.stake_token_mint.as_ref(),
            &[pda_bump]
        ];
        msg!("About to send tokens");
        invoke_signed(
            &token_transfer_ix,
            &[
                contract_token_account_info.clone(),
                user_token_account_info.clone(),
                contract_data_account.clone(),
                token_program_info.clone(),
            ],
            &[signer_seeds],
        )?;
        msg!("Sent tokens");
        // Reset User Account and Contract Account
        contract_data.total_staked = contract_data.total_staked.saturating_sub(user_data.total_staked);
        user_data.total_staked = 0;
        user_data.interest_accrued = 0;
        user_data.stake_ts = 0;

        UserData::pack(user_data, &mut user_data_account.try_borrow_mut_data()?)?;
        ContractData::pack(contract_data, &mut contract_data_account.try_borrow_mut_data()?)?;
        Ok(())
    }
    fn perform_staking<'a>(
        program_id: &Pubkey,
        user_info: &AccountInfo<'a>,
        user_token_account_info: &AccountInfo<'a>,
        user_data_account: &AccountInfo<'a>,
        system_program_info: &AccountInfo<'a>,
        token_program_info: &AccountInfo<'a>,
        contract_token_account_info: &AccountInfo<'a>,
        contract_data_account: &AccountInfo<'a>,
        stake_type: StakeType,
        amount: u64,
        apy: u64,
        lock_duration: u64
    ) -> ProgramResult {
        // verify the user data account
        let seeds: &[&[u8]] = &[b"spl_staking_user", user_info.key.as_ref()];
        let (ns_user_data_pda, bump) = Pubkey::find_program_address(
            seeds,
            program_id
        );
        if *user_data_account.key != ns_user_data_pda {
            msg!("Staking [Error]: User data account and generated pda mismatch");
            return Err(ProgramError::InvalidAccountData.into())
        }

        let clock = Clock::get()?;
        let current_ts = clock.unix_timestamp as u64;
        let mut contract_data = ContractData::unpack_unchecked(&contract_data_account.data.borrow())?;
        let mut user_data = if user_data_account.data_len() == 0 {
            // Create the PDA Account
            let rent = &Rent::get()?;
            let required_lamports = rent
                .minimum_balance(UserData::LEN)
                .max(1)
                .saturating_sub(user_data_account.lamports());
            let signer_seeds: &[&[u8]] = &[b"spl_staking_user", user_info.key.as_ref(), &[bump]];
            invoke_signed(
                &system_instruction::create_account(
                    user_info.key,
                    &ns_user_data_pda,
                    required_lamports,
                    UserData::LEN as u64,
                    program_id,
                ),
                &[
                    user_info.clone(),
                    user_data_account.clone(),
                    system_program_info.clone(),
                ],
                &[signer_seeds],
            )?;
            let mut data = UserData::unpack_unchecked(
                &user_data_account.data.borrow()
            )?;
            data.stake_type = stake_type;
            data.owner_pubkey = *user_info.key;
            data.is_initialized = false;
            data.total_staked = 0;
            data.interest_accrued = 0;
            data.last_claim_ts = 0;
            data.last_unstake_ts = 0;
            data.lock_duration = lock_duration;
            data.stake_ts = current_ts;
            data
        } else {
            UserData::unpack_from_slice(
                &user_data_account.data.borrow()
            )?
        };
        // First time staking
        if !user_data.is_initialized {
            msg!("Staking [Info]: First time staking");
            // Transfer tokens to contract pda for locking
            let transfer_tkn_ix = spl_token::instruction::transfer(
                &spl_token::ID,
                user_token_account_info.key,
                contract_token_account_info.key,
                user_info.key,
                &[user_info.key],
                amount
            )?;
            invoke(
                &transfer_tkn_ix,
                &[
                    user_token_account_info.clone(),
                    contract_token_account_info.clone(),
                    user_info.clone(),
                    token_program_info.clone()
                ]
            )?;
            user_data.is_initialized = true;
            user_data.total_staked = amount;
            contract_data.total_staked = contract_data.total_staked.add(amount);
        } else {
            msg!("Staking [Info]: Re-staking");
            // Transfer tokens to contract pda
            let transfer_tkn_ix = spl_token::instruction::transfer(
                &spl_token::ID,
                user_token_account_info.key,
                contract_token_account_info.key,
                user_info.key,
                &[user_info.key],
                amount
            )?;
            invoke(
                &transfer_tkn_ix,
                &[
                    user_token_account_info.clone(),
                    contract_token_account_info.clone(),
                    user_info.clone(),
                    token_program_info.clone()
                ]
            )?;
            // Calculate the interest accrued from stake_ts till now
            let stake_interval = current_ts - user_data.stake_ts;
            let interest_accrued = (apy * user_data.total_staked * stake_interval)/31536000000;
            msg!("Staking[Info]: Interest Accrued: {}\nStake Interval: {}", interest_accrued, stake_interval);
            user_data.interest_accrued = user_data.interest_accrued.add(interest_accrued);
            user_data.total_staked = user_data.total_staked.add(amount);
            user_data.stake_ts = current_ts;
            contract_data.total_staked = contract_data.total_staked.add(amount);
            contract_data.total_earned = contract_data.total_earned.add(interest_accrued);
        }
        UserData::pack(user_data, &mut user_data_account.try_borrow_mut_data()?)?;
        ContractData::pack(contract_data, &mut contract_data_account.try_borrow_mut_data()?)?;
        Ok(())
    }
}