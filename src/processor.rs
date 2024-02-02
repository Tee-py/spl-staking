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
                early_withdrawal_fee
            } => {
                msg!("Staking [Info]: Init contract instruction");
                Self::init(
                    program_id, accounts,
                    minimum_stake_amount, minimum_lock_duration,
                    normal_staking_apy, locked_staking_apy,
                    early_withdrawal_fee
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
        early_withdrawal_fee: u64
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
        _lock_duration: u64
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let user_info = next_account_info(account_info_iter)?;
        let user_token_account_info = next_account_info(account_info_iter)?;
        let user_data_account_info = next_account_info(account_info_iter)?;
        let contract_token_account_info = next_account_info(account_info_iter)?;
        let contract_data_account_info = next_account_info(account_info_iter)?;
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
            StakeType::NORMAL => Self::perform_normal_staking(
                program_id,
                user_info,
                user_token_account_data,
                user_data_account_info,
                system_program_info,
                contract_token_account_data,
                contract_data,
                amount
            ),
            StakeType::LOCKED => {msg!("Staking [Info]: Locked Staking");Ok(())}
        }
    }
    fn perform_normal_staking<'a>(
        program_id: &Pubkey,
        user_info: &AccountInfo<'a>,
        _user_token_account: TokenAccount,
        user_data_account: &AccountInfo<'a>,
        system_program_info: &AccountInfo<'a>,
        _contract_token_account: TokenAccount,
        _contract_data: ContractData,
        _amount: u64
    ) -> ProgramResult {
        msg!("Staking [Info]: Performing Normal Staking");
        // verify the user data account
        let seeds: &[&[u8]] = &[b"spl_staking_normal_user", user_info.key.as_ref()];
        let (ns_user_data_pda, bump) = Pubkey::find_program_address(
            seeds,
            program_id
        );
        if *user_data_account.key != ns_user_data_pda {
            msg!("Staking [Error]: User data account and generated pda mismatch");
            return Err(ProgramError::InvalidAccountData.into())
        }

        let user_data = if user_data_account.data_len() == 0 {
            // Create the PDA Account
            let rent = &Rent::get()?;
            let required_lamports = rent
                .minimum_balance(UserData::LEN)
                .max(1)
                .saturating_sub(user_data_account.lamports());
            let signer_seeds: &[&[u8]] = &[b"spl_staking_normal_user", user_info.key.as_ref(), &[bump]];
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
            let clock = Clock::get()?;
            let current_ts = clock.unix_timestamp as u64;
            data.stake_type = StakeType::NORMAL;
            data.owner_pubkey = *user_info.key;
            data.is_initialized = false;
            data.total_staked = 0;
            data.interest_accrued = 0;
            data.last_claim_ts = 0;
            data.last_unstake_ts = 0;
            data.lock_duration = 0;
            data.stake_ts = current_ts;

            data
        } else {
            UserData::unpack_from_slice(
                &user_data_account.data.borrow()
            )?
        };
        UserData::pack(user_data, &mut user_data_account.try_borrow_mut_data()?)?;
        Ok(())
    }
}