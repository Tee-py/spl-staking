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
use solana_program::rent::Rent;
//use spl_token::state::{Account as TokenAccount, Mint};
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
        let mut contract_data = ContractData::unpack_unchecked(&data_account.data.borrow())?;
        contract_data.is_initialized = true;
        contract_data.admin_pubkey = *admin.key;
        contract_data.stake_token_mint = *mint_info.key;
        contract_data.minimum_stake_amount = minimum_stake_amount;
        contract_data.minimum_lock_duration = minimum_lock_duration;
        contract_data.stake_token_account = *token_account.key;

        ContractData::pack(contract_data, &mut data_account.try_borrow_mut_data()?)?;
        Ok(())
    }
}