use soroban_sdk::{contracttype, Address};

pub(crate) const DAY_IN_LEDGERS: u32 = 17280;
pub(crate) const INSTANCE_BUMP_AMOUNT: u32 = 7 * DAY_IN_LEDGERS;
pub(crate) const INSTANCE_LIFETIME_THRESHOLD: u32 = INSTANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;

pub(crate) const BALANCE_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub(crate) const BALANCE_LIFETIME_THRESHOLD: u32 = BALANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;

#[derive(Clone)]
#[contracttype]
pub struct AllowanceDataKey {
    pub from: Address,
    pub spender: Address,
}

#[contracttype]
pub struct AllowanceValue {
    pub amount: i128,
    pub expiration_ledger: u32,
}

#[contracttype]
pub struct ProjectInfo {
    pub borrower: Address,
    pub lend_token_address: Address,
    pub collateral_nft_address: Address,
    pub collateral_id: u128,
    pub target_amount: i128,
    pub start_timestamp: u64,
    pub final_timestamp: u64,
    pub reward_rate: i128,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Allowance(AllowanceDataKey),
    Balance(Address),
    Nonce(Address),
    State(Address),
    Admin,
    ProjectInfo,
    TotalSupply,
    NumberOfLenders,
    LenderIndex(Address),
    LenderAddress(u128),
    ClaimAvailable,
    ClaimedBalance(Address),
    TotalReturn,
    FeeAccumulated,
    TargetNotReached,
    BorrowerClaimed,
}
