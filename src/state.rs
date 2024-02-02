use solana_program::{
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey
};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::program_error::ProgramError;


/// Struct for packing and unpacking contract data
///
/// Fields [All are Public]
///
/// 1. is_initialized [boolean]: boolean
/// 2. admin_pubkey [Pubkey]: Address of the initializer of the smart contract
/// 3. stake_token_mint [Pubkey]: Address of the token to be staked
/// 4. minimum_stake_amount [u64]: Minimum number of tokens allowed for staking(in decimals format)
/// 5. minimum_lock_duration [u64]: Minimum duration for token lock in seconds
pub struct ContractData {
    pub is_initialized: bool,
    pub admin_pubkey: Pubkey,
    pub stake_token_mint: Pubkey,
    pub stake_token_account: Pubkey,
    pub minimum_stake_amount: u64,
    pub minimum_lock_duration: u64
}

impl Sealed for ContractData {}

impl IsInitialized for ContractData {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl ContractData {
    pub const LEN: usize = 1 + 32 + 32 + 8 + 8;
}

impl Pack for ContractData {
    const LEN: usize = ContractData::LEN;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, ContractData::LEN];
        let (
            init_state_dst,
            admin_pk_dst,
            stake_tkn_dst,
            min_stake_dst,
            min_lk_dst
        ) = mut_array_refs![dst, 1, 32, 32, 8, 8];
        init_state_dst[0] = *self.is_initialized as u8;
        admin_pk_dst.copy_from_slice(self.admin_pubkey.as_ref());
        stake_tkn_dst.copy_from_slice(self.stake_token_mint.as_ref());
        *min_stake_dst = self.minimum_stake_amount.to_le_bytes();
        *min_lk_dst = self.minimum_lock_duration.to_le_bytes();
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, ContractData::LEN];
        let (
            init_dst,
            admin_pk_dst,
            stake_tkn_dst,
            min_stake_dst,
            min_lk_dst
        ) = array_refs![src, 1, 32, 32, 8, 8];
        let is_initialized = match init_dst[0] {
            0 => false,
            1 => true,
            _ => return Err(ProgramError::InvalidAccountData.into())
        };
        Ok(ContractData {
            is_initialized,
            admin_pubkey: Pubkey::new_from_array(*admin_pk_dst),
            stake_token_mint: Pubkey::new_from_array(*stake_tkn_dst),
            minimum_stake_amount: u64::from_le_bytes(*min_stake_dst),
            minimum_lock_duration: u64::from_le_bytes(*min_lk_dst)
        })
    }
}