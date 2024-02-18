use arrayref::{array_ref, array_refs};
use solana_program::program_error::ProgramError;
use crate::state::StakeType;


pub enum Instruction {
    /// Initialize the staking contract by setting necessary states needed for the contract
    ///
    /// Accounts Expected
    ///
    /// 1. `[signer]` The admin of the contract
    /// 2. `[writable]` The data account for the contract which is a PDA
    /// 3. `[writable]` The token account for storing reward and staked tokens [A PDA]
    /// 4. `[]` The stake token mint address
    /// 5. `[]` Token program address
    /// 6. `[]` Rent info
    /// 7. `[]` system program
    Init {
        /// Minimum amount of tokens to be staked
        minimum_stake_amount: u64,
        /// Minimum amount of time interval(in seconds) for locking
        minimum_lock_duration: u64,
        /// APY For normal staking (decimals = 1)
        normal_staking_apy: u64,
        /// APY For locked staking (decimals = 1)
        locked_staking_apy: u64,
        /// Penalty for early withdrawal in locked staking (decimals = 1)
        early_withdrawal_fee: u64,
        /// percentage tax for TOKEN_2022 (decimals = 1)
        tax_percent: u64
    },

    /// Stake tokens
    ///
    /// Accounts Expected
    ///
    /// 1. `[Signer]` The user signer
    /// 2. `[writable]` The token account of the user
    /// 3. `[writable]` The user data account for the contract
    /// 4. `[writable]` The token account for the contract
    /// 6. `[writable]` The data account for the contract
    /// 7. `[]` System program info
    Stake {
        stake_type: StakeType,
        amount: u64,
        lock_duration: u64
    },

    /// Unstake tokens
    ///
    /// Accounts Expected
    ///
    /// 1. `[Signer]` The user signer
    /// 2. `[writable]` The token account of the user
    /// 3. `[writable]` The user data account for the contract
    /// 4. `[writable]` The token account for the contract
    /// 5. `[writable]` The data account for the contract
    UnStake,

    /// Change percentage tax for token 2022 mint
    ///
    /// Accounts Expected
    ///
    /// 1. `[Signer]` The admin of the contract data account
    /// 2. `[writable]` The contract data account
    ChangeTaxPercent {
        tax_percent: u64
    }
}

impl Instruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (tag, rest) = input.split_first().ok_or(ProgramError::InvalidInstructionData)?;
        Ok(
            match tag {
                0 => {
                    let rest = array_ref![rest, 0, 40];
                    let (
                        min_stk_dst,
                        min_lk_dst,
                        ns_apy_dst,
                        ls_apy_dst,
                        e_wdf_dst,
                        tax_per
                    ) = array_refs![rest, 8, 8, 8, 8, 8, 8];
                    Self::Init {
                        minimum_stake_amount: Self::unpack_u64(min_stk_dst)?,
                        minimum_lock_duration: Self::unpack_u64(min_lk_dst)?,
                        normal_staking_apy: Self::unpack_u64(ns_apy_dst)?,
                        locked_staking_apy: Self::unpack_u64(ls_apy_dst)?,
                        early_withdrawal_fee: Self::unpack_u64(e_wdf_dst)?,
                        tax_percent: Self::unpack_u64(tax_per)?,
                    }
                },
                1 => {
                    let rest = array_ref![rest, 0, 17];
                    let (
                        stake_type_dst,
                        amount_dst,
                        lock_duration_dst
                    ) = array_refs![rest, 1, 8, 8];
                    let stake_type = match stake_type_dst[0] {
                        0 => StakeType::NORMAL,
                        1 => StakeType::LOCKED,
                        _ => return Err(ProgramError::InvalidInstructionData.into())
                    };
                    Self::Stake {
                        stake_type,
                        amount: Self::unpack_u64(amount_dst)?,
                        lock_duration: Self::unpack_u64(lock_duration_dst)?
                    }
                },
                2 => {
                    Self::UnStake
                },
                3 => {
                    Self::ChangeTaxPercent {
                        tax_percent: Self::unpack_u64(rest)?
                    }
                }
                _ => {
                    return Err(ProgramError::InvalidInstructionData.into())
                },
            }
        )
    }

    fn unpack_u64(input: &[u8]) -> Result<u64, ProgramError> {
        let value = input
            .get(..8)
            .and_then(|slice| slice.try_into().ok())
            .map(u64::from_le_bytes)
            .ok_or(ProgramError::InvalidInstructionData)?;
        Ok(value)
    }
}