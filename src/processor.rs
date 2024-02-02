use solana_program::{
    account_info::{AccountInfo, next_account_info},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_pack::{Pack},
    pubkey::Pubkey,
    system_instruction,
    sysvar::{Sysvar},
    program::{invoke_signed, invoke},
    clock::Clock,
    msg,
};
use solana_program::rent::Rent;
use spl_token::state::{Account as TokenAccount, Mint};
use crate::instruction::Instruction as ContractInstruction;
use crate::state::{ContractData};


pub struct Processor;

impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8]
    ) -> ProgramResult {
        let instruction = ContractInstruction::unpack(instruction_data)?;
        match instruction {
            ContractInstruction::Init { minimum_stake_amount, minimum_lock_duration } => {
                msg!("Staking [Info]: Init contract instruction");
                Self::init(program_id, accounts, minimum_stake_amount, minimum_lock_duration)
            },
        }
    }

    fn init(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        minimum_stake_amount: u64,
        minimum_lock_duration: u64
    ) -> ProgramResult {
        // Get all accounts sent to the instruction
        let accounts_info_iter = &mut accounts.iter();
        let admin = next_account_info(accounts_info_iter)?;
        let data_account = next_account_info(accounts_info_iter)?;
        let mint_token_account = next_account_info(accounts_info_iter)?;
        let mint_account = next_account_info(accounts_info_iter)?;
        let token_program_account = next_account_info(accounts_info_iter)?;
        let rent_info = next_account_info(accounts_info_iter)?;
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
            mint_account.key.as_ref()
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
        let contract_seeds: &[&[u8]] = &[b"spl_staking", admin.key.as_ref(), mint_account.key.as_ref(), &[pda_bump]];
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
                mint_account.clone(),
                system_program_account.clone(),
            ],
            &[contract_seeds],
        )?;

        // Create a token account to store staked tokens and rewards
        let token_acct_seeds: &[&[u8]] = &[
            b"spl_staking_token_account",
            admin.key.as_ref()
        ];
        let (pda_token_acct, bump) = Pubkey::find_program_address(token_acct_seeds, program_id);
        if &pda_token_acct != mint_token_account.key {
            msg!("Staking [Error]: Derived token account address mismatch");
            return Err(ProgramError::InvalidAccountData.into())
        }
        let required_lamports = rent
            .minimum_balance(TokenAccount::LEN)
            .max(1);
        let create_acct_ix = system_instruction::create_account(
            admin.key,
            &pda_token_acct,
            required_lamports,
            ContractData::LEN as u64,
            program_id,
        );
        let signer_seeds: &[&[u8]] = &[b"spl_staking_token_account", admin.key.as_ref(), &[bump]];
        invoke_signed(
            &create_acct_ix,
            &[admin.clone(), mint_token_account.clone()],
            &[signer_seeds]
        )?;
        let init_acct_ix = spl_token::instruction::initialize_account(
            &spl_token::ID,
            &pda_token_acct,
            mint_account.key,
            data_account.key
        )?;
        invoke_signed(
            &init_acct_ix,
            &[
                mint_token_account.clone(),
                mint_account.clone(),
                data_account.clone(),
                rent_info.clone(),
                token_program_account.clone()
            ],
            &[signer_seeds]
        )?;
        let mut contract_data = ContractData::unpack_unchecked(&data_account.data.borrow())?;
        contract_data.is_initialized = true;
        contract_data.admin_pubkey = *admin.key;
        contract_data.stake_token_mint = *mint_account.key;
        contract_data.minimum_stake_amount = minimum_stake_amount;
        contract_data.minimum_lock_duration = minimum_lock_duration;
        contract_data.stake_token_account = pda_token_acct;

        ContractData::pack(contract_data, &mut data_account.try_borrow_mut_data()?)?;
        Ok(())
    }
}