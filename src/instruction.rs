use arrayref::{array_ref, array_refs};
use solana_program::program_error::ProgramError;


pub enum Instruction {
    /// Initialize the staking contract by setting necessary states needed for the contract
    ///
    /// Accounts Expected
    ///
    /// 1. `[signer]` The admin of the contract
    /// 2. `[writable]` The data account for the contract which is a PDA
    /// 3. `[writable]` The token account for storing reward and staked tokens [A PDA]
    /// 4. `[]` The stake token mint address
    /// 5. `[]` system program
    Init {
        /// Minimum amount of tokens to be staked
        minimum_stake_amount: u64,
        /// Minimum amount of time interval(in seconds) for locking
        minimum_lock_duration: u64
    }
}

impl Instruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (tag, rest) = input.split_first().ok_or(ProgramError::InvalidInstructionData)?;
        Ok(
            match tag {
                0 => {
                    let rest = array_ref![rest, 0, 16];
                    let (part1, part2) = array_refs![rest, 8, 8];
                    Self::Init {
                        minimum_stake_amount: Self::unpack_u64(part1)?,
                        minimum_lock_duration: Self::unpack_u64(part2)?
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