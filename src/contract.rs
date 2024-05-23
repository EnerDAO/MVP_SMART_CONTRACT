//! EnerDAO tokenized funding
use crate::admin::{has_administrator, read_administrator, write_administrator};
use crate::allowance::{read_allowance, spend_allowance, write_allowance};
use crate::balance::{read_balance, receive_balance, spend_balance};
use crate::metadata::{read_decimal, read_name, read_symbol, write_metadata};
#[cfg(test)]
use crate::storage_types::{AllowanceDataKey, AllowanceValue};
use crate::storage_types::{
    DataKey, ProjectInfo, BALANCE_BUMP_AMOUNT, BALANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT,
    INSTANCE_LIFETIME_THRESHOLD,
};
use soroban_sdk::token::{self, Interface as _};
use soroban_sdk::{contract, contractimpl, panic_with_error, Address, Env, String, Vec};
use soroban_token_sdk::metadata::TokenMetadata;
use soroban_token_sdk::TokenUtils;

use crate::errors::Error;

const PROTOCOL_FEE: i128 = 1000;
const REWARD_DENOM: i128 = 10000;

mod contract_nft {
    soroban_sdk::contractimport!(file = "./token/non_fungible_token.optimized.wasm");
}

fn check_nonnegative_amount(e: &Env, amount: i128) {
    if amount < 0 {
        panic_with_error!(e, Error::OnlyPositiveValue);
    }
}

fn get_project_info(e: &Env) -> ProjectInfo {
    let key = DataKey::ProjectInfo;
    let project_info: ProjectInfo = e.storage().persistent().get(&key).unwrap();
    return project_info;
}

fn require_start_time_reached(e: &Env) {
    let start_time: u64 = get_project_info(e).start_timestamp;
    if e.ledger().timestamp() < start_time {
        panic_with_error!(e, Error::NotStarted)
    }
}

fn require_final_time_not_reached(e: &Env) {
    let final_time: u64 = get_project_info(e).final_timestamp;
    if e.ledger().timestamp() > final_time {
        panic_with_error!(e, Error::AlreadyFinished)
    }
}

fn require_final_time_reached(e: &Env) {
    let final_time: u64 = get_project_info(e).final_timestamp;
    if e.ledger().timestamp() <= final_time {
        panic_with_error!(e, Error::NotFinished)
    }
}

fn require_target_amount_reached(e: &Env) {
    let target_amount: i128 = get_project_info(e).target_amount;
    if read_total_supply(&e) < target_amount {
        panic_with_error!(e, Error::TargetNotReached)
    }
}

fn require_nft_collateral(e: &Env) {
    let collateral_nft_address: Address = get_project_info(e).collateral_nft_address;
    let collateral_id: u128 = get_project_info(e).collateral_id;
    // owner_of() &e.current_contract_address()
    let nft_client = contract_nft::Client::new(&e, &collateral_nft_address);
    let nft_has_owner: bool = nft_client.has_owner(&collateral_id);
    if !nft_has_owner {
        panic_with_error!(e, Error::NoCollateral)
    }
    let nft_owner: Address = nft_client.owner_of(&collateral_id);
    if nft_owner != e.current_contract_address() {
        panic_with_error!(e, Error::NoCollateral)
    }
}

fn read_total_supply(e: &Env) -> i128 {
    let key = DataKey::TotalSupply;
    let total_supply: i128 = e.storage().persistent().get(&key).unwrap_or(0);
    total_supply
}

fn write_total_supply(e: &Env, val: i128) {
    let key = DataKey::TotalSupply;
    e.storage().persistent().set(&key, &val);
    e.storage()
        .persistent()
        .extend_ttl(&key, BALANCE_LIFETIME_THRESHOLD, BALANCE_BUMP_AMOUNT);
}

fn add_total_supply(e: &Env, val: i128) {
    let mut total_supply: i128 = read_total_supply(e);
    total_supply += val;
    write_total_supply(e, total_supply);
}

fn sub_total_supply(e: &Env, val: i128) {
    let mut total_supply: i128 = read_total_supply(e);
    total_supply -= val;
    write_total_supply(e, total_supply);
}

fn _mint(e: Env, to: Address, amount: i128) {
    let admin = read_administrator(&e);

    e.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

    receive_balance(&e, to.clone(), amount);
    add_total_supply(&e, amount);
    TokenUtils::new(&e).events().mint(admin, to, amount);
}

fn _burn(e: Env, from: Address, amount: i128) {
    e.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

    spend_balance(&e, from.clone(), amount);
    sub_total_supply(&e, amount);
    TokenUtils::new(&e).events().burn(from, amount);
}

fn read_number_of_lenders(e: &Env) -> u128 {
    let key = DataKey::NumberOfLenders;
    let number_of_lenders: u128 = e.storage().persistent().get(&key).unwrap_or(0);
    number_of_lenders
}

fn write_number_of_lenders(e: &Env, val: u128) {
    let key = DataKey::NumberOfLenders;
    e.storage().persistent().set(&key, &val);
    e.storage()
        .persistent()
        .extend_ttl(&key, BALANCE_LIFETIME_THRESHOLD, BALANCE_BUMP_AMOUNT);
}

// internal function that records index of the lender
// if this is a new lender
fn _add_lender(e: Env, lender: Address) {
    let mut number_of_lenders: u128 = read_number_of_lenders(&e);
    let mut lender_index: u128 = e
        .storage()
        .persistent()
        .get(&DataKey::LenderIndex(lender.clone()))
        .unwrap_or(0);

    if lender_index == 0 {
        number_of_lenders += 1;
        lender_index = number_of_lenders;
        write_number_of_lenders(&e, number_of_lenders);
        e.storage()
            .persistent()
            .set(&DataKey::LenderIndex(lender.clone()), &lender_index);
        e.storage()
            .persistent()
            .set(&DataKey::LenderAddress(lender_index), &lender);
    }
}

#[contract]
pub struct EnerDAOToken;

fn move_token(env: &Env, from: &Address, to: &Address, transfer_amount: i128) {
    let token: Address = get_project_info(env).lend_token_address;
    // token interface
    let token_client: token::TokenClient<'_> = token::Client::new(&env, &token);
    token_client.transfer(&from, to, &transfer_amount);
}

#[contractimpl]
impl EnerDAOToken {
    pub fn initialize(e: Env, admin: Address, decimal: u32, name: String, symbol: String) {
        if has_administrator(&e) {
            panic_with_error!(&e, Error::AlreadyInitialized)
        }
        write_administrator(&e, &admin);

        write_metadata(
            &e,
            TokenMetadata {
                decimal,
                name,
                symbol,
            },
        );
    }

    pub fn init_project(
        e: Env,
        borrower: Address,
        lend_token_address: Address,
        collateral_nft_address: Address,
        collateral_id: u128,
        target_amount: i128,
        start_timestamp: u64,
        final_timestamp: u64,
        reward_rate: i128,
    ) {
        let admin = read_administrator(&e);
        admin.require_auth();

        let project_info: ProjectInfo = ProjectInfo {
            borrower,
            lend_token_address,
            collateral_nft_address,
            collateral_id,
            target_amount,
            start_timestamp,
            final_timestamp,
            reward_rate,
        };
        let project_key: DataKey = DataKey::ProjectInfo;
        e.storage().persistent().set(&project_key, &project_info);
    }

    pub fn lend(e: Env, lender: Address, amount: i128) {
        check_nonnegative_amount(&e, amount);
        lender.require_auth();

        require_start_time_reached(&e);
        require_final_time_not_reached(&e);

        let target_amount: i128 = get_project_info(&e).target_amount;
        let total_supply: i128 = read_total_supply(&e);
        if total_supply + amount > target_amount {
            panic_with_error!(e, Error::TargetOverreached)
        }

        move_token(&e, &lender, &e.current_contract_address(), amount);
        _mint(e.clone(), lender.clone(), amount);
        _add_lender(e.clone(), lender.clone());
    }

    pub fn is_lender_claim_available(e: &Env) -> bool {
        let key: DataKey = DataKey::ClaimAvailable;
        let claim_available: bool = e.storage().persistent().get(&key).unwrap_or(false);
        return claim_available;
    }

    pub fn lender_available_to_claim(e: Env, lender: Address) -> i128 {
        if !Self::is_lender_claim_available(&e) {
            return 0;
        }

        let lender_balance: i128 = read_balance(&e, lender.clone());

        let target_not_reached = e
            .storage()
            .persistent()
            .get(&DataKey::TargetNotReached)
            .unwrap_or(false);
        if target_not_reached {
            return lender_balance;
        }

        let key_claimed: DataKey = DataKey::ClaimedBalance(lender.clone());
        let already_claimed: i128 = e.storage().persistent().get(&key_claimed).unwrap_or(0);

        let key_return: DataKey = DataKey::TotalReturn;
        let total_return: i128 = e.storage().persistent().get(&key_return).unwrap_or(0);
        let target_amount: i128 = get_project_info(&e).target_amount;

        let total_available_to_claim: i128 =
            total_return * (lender_balance + already_claimed) / target_amount;

        let reward_rate: i128 = get_project_info(&e).reward_rate;

        let available_to_claim = total_available_to_claim
            - already_claimed * (reward_rate + REWARD_DENOM) / REWARD_DENOM;

        // Rounding issue
        if available_to_claim == 1 {
            return 0;
        } else {
            return available_to_claim;
        }
    }

    pub fn lender_claim(e: Env, lender: Address) {
        lender.require_auth();

        let entitled_amount: i128 = Self::lender_available_to_claim(e.clone(), lender.clone());

        if entitled_amount <= 0 {
            panic_with_error!(e, Error::NothingToClaim)
        }
        let reward_rate: i128 = get_project_info(&e).reward_rate;
        let target_not_reached: bool = e
            .storage()
            .persistent()
            .get(&DataKey::TargetNotReached)
            .unwrap_or(false);

        let burn_amount: i128;
        if target_not_reached {
            burn_amount = entitled_amount;
        } else {
            burn_amount = entitled_amount * REWARD_DENOM / (REWARD_DENOM + reward_rate);
        }

        _burn(e.clone(), lender.clone(), burn_amount);

        move_token(&e, &e.current_contract_address(), &lender, entitled_amount);

        let key_claimed: DataKey = DataKey::ClaimedBalance(lender.clone());
        let mut already_claimed: i128 = e.storage().persistent().get(&key_claimed).unwrap_or(0);
        already_claimed += burn_amount;
        e.storage().persistent().set(&key_claimed, &already_claimed);
    }

    pub fn borrower_claim(e: Env) {
        let borrower: Address = get_project_info(&e).borrower;
        borrower.require_auth();

        let already_claimed: bool = e.storage().persistent().get(&DataKey::BorrowerClaimed).unwrap_or(false);
        if already_claimed {
            panic_with_error!(&e, Error::AlreadyClaimed)
        }

        require_target_amount_reached(&e);
        // require_final_time_reached(&e);
        require_nft_collateral(&e);

        e.storage().persistent().set(&DataKey::BorrowerClaimed, &true);

        let amount: i128 = read_total_supply(&e);
        move_token(&e, &e.current_contract_address(), &borrower, amount);
    }

    pub fn borrower_claim_status(e: &Env) -> String {

        let already_claimed: bool = e.storage().persistent().get(&DataKey::BorrowerClaimed).unwrap_or(false);
        if already_claimed {
            return String::from_str(e, "AlreadyClaimed");
        }

        let project_info: ProjectInfo = get_project_info(e);
        let target_amount: i128 = project_info.target_amount;
        let final_time: u64 = project_info.final_timestamp;
        // if e.ledger().timestamp() <= final_time {
        //     return String::from_str(e, "NotFinished");
        // }
        if read_total_supply(&e) < target_amount {
            return String::from_str(e, "TargetNotReached");
        }
        let collateral_nft_address: Address = project_info.collateral_nft_address;
        let collateral_id: u128 = project_info.collateral_id;
        let nft_client = contract_nft::Client::new(&e, &collateral_nft_address);
        let nft_has_owner: bool = nft_client.has_owner(&collateral_id);
        if !nft_has_owner {
            return String::from_str(e, "NoCollateral");
        }
        let nft_owner: Address = nft_client.owner_of(&collateral_id);
        if nft_owner != e.current_contract_address() {
            return String::from_str(e, "NoCollateral");
        }
        return String::from_str(e, "Available");
    }

    pub fn borrower_return(e: Env, borrower: Address, amount: i128) {
        borrower.require_auth();

        require_final_time_reached(&e);

        move_token(&e, &borrower, &e.current_contract_address(), amount);

        // Calculation of protocol fee
        let reward_rate: i128 = get_project_info(&e).reward_rate;
        let base_return: i128 = amount * REWARD_DENOM
            / (REWARD_DENOM + reward_rate + reward_rate * PROTOCOL_FEE / REWARD_DENOM);
        let protocol_fee: i128 = amount - base_return - base_return * reward_rate / REWARD_DENOM;

        let key_return: DataKey = DataKey::TotalReturn;
        let mut total_return: i128 = e.storage().persistent().get(&key_return).unwrap_or(0);
        total_return += amount - protocol_fee;
        e.storage().persistent().set(&key_return, &total_return);

        let key_fee: DataKey = DataKey::FeeAccumulated;
        let mut total_fee = e.storage().persistent().get(&key_fee).unwrap_or(0);
        total_fee += protocol_fee;
        e.storage().persistent().set(&key_fee, &total_fee);

        let key_claim: DataKey = DataKey::ClaimAvailable;
        e.storage().persistent().set(&key_claim, &true);
    }

    pub fn mint(e: Env, to: Address, amount: i128) {
        check_nonnegative_amount(&e, amount);
        let admin = read_administrator(&e);
        admin.require_auth();

        _mint(e, to, amount);
    }

    pub fn set_admin(e: Env, new_admin: Address) {
        let admin = read_administrator(&e);
        admin.require_auth();

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        write_administrator(&e, &new_admin);
        TokenUtils::new(&e).events().set_admin(admin, new_admin);
    }

    pub fn set_project_info(e: Env, new_project_info: ProjectInfo) {
        let admin = read_administrator(&e);
        admin.require_auth();

        let project_key: DataKey = DataKey::ProjectInfo;
        e.storage()
            .persistent()
            .set(&project_key, &new_project_info);
    }

    pub fn set_lender_claim_available(e: Env, is_available: bool, target_not_reached: bool) {
        let admin = read_administrator(&e);
        admin.require_auth();

        let key_claim: DataKey = DataKey::ClaimAvailable;
        e.storage().persistent().set(&key_claim, &is_available);

        let key_target: DataKey = DataKey::TargetNotReached;
        e.storage()
            .persistent()
            .set(&key_target, &target_not_reached);
    }

    pub fn grant_nft(e: Env, to: Address) {
        let admin = read_administrator(&e);
        admin.require_auth();

        let collateral_nft_address: Address = get_project_info(&e).collateral_nft_address;
        let collateral_id: u128 = get_project_info(&e).collateral_id;
        // transfer NFT
        let nft_client = contract_nft::Client::new(&e, &collateral_nft_address);
        nft_client.transfer(&e.current_contract_address(), &to, &collateral_id);
    }

    pub fn rescue_tokens(e: Env, token_address: Address, to: Address, amount: i128) {
        let admin = read_administrator(&e);
        admin.require_auth();

        let token_client: token::TokenClient<'_> = token::Client::new(&e, &token_address);
        token_client.transfer(&e.current_contract_address(), &to, &amount);
    }

    pub fn get_project_info(e: Env) -> ProjectInfo {
        get_project_info(&e)
    }

    pub fn total_return(e: Env) -> i128 {
        let key_return: DataKey = DataKey::TotalReturn;
        e.storage().persistent().get(&key_return).unwrap_or(0)
    }

    pub fn fee_accumulated(e: Env) -> i128 {
        let key_fee: DataKey = DataKey::FeeAccumulated;
        e.storage().persistent().get(&key_fee).unwrap_or(0)
    }

    pub fn total_supply(e: Env) -> i128 {
        read_total_supply(&e)
    }

    pub fn number_of_lenders(e: Env) -> u128 {
        read_number_of_lenders(&e)
    }

    pub fn get_lenders(e: Env) -> Vec<Address> {
        let mut lenders: Vec<Address> = Vec::<Address>::new(&e);
        for i in 1..=read_number_of_lenders(&e) {
            let user_address: Address = e
                .storage()
                .persistent()
                .get(&DataKey::LenderAddress(i))
                .unwrap();
            lenders.push_back(user_address);
        }
        lenders
    }

    #[cfg(test)]
    pub fn get_allowance(e: Env, from: Address, spender: Address) -> Option<AllowanceValue> {
        let key = DataKey::Allowance(AllowanceDataKey { from, spender });
        let allowance = e.storage().temporary().get::<_, AllowanceValue>(&key);
        allowance
    }
}

#[contractimpl]
impl token::Interface for EnerDAOToken {
    fn allowance(e: Env, from: Address, spender: Address) -> i128 {
        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        read_allowance(&e, from, spender).amount
    }

    fn approve(e: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
        panic_with_error!(&e, Error::NotAllowed);
        // from.require_auth();

        // check_nonnegative_amount(&e, amount);

        // e.storage()
        //     .instance()
        //     .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        // write_allowance(&e, from.clone(), spender.clone(), amount, expiration_ledger);
        // TokenUtils::new(&e)
        //     .events()
        //     .approve(from, spender, amount, expiration_ledger);
    }

    fn balance(e: Env, id: Address) -> i128 {
        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        read_balance(&e, id)
    }

    fn transfer(e: Env, from: Address, to: Address, amount: i128) {
        panic_with_error!(&e, Error::NotAllowed);
        // from.require_auth();

        // check_nonnegative_amount(&e, amount);

        // e.storage()
        //     .instance()
        //     .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        // spend_balance(&e, from.clone(), amount);
        // receive_balance(&e, to.clone(), amount);
        // _add_lender(e.clone(), to.clone());
        // TokenUtils::new(&e).events().transfer(from, to, amount);
    }

    fn transfer_from(e: Env, spender: Address, from: Address, to: Address, amount: i128) {
        panic_with_error!(&e, Error::NotAllowed);
        // spender.require_auth();

        // check_nonnegative_amount(&e, amount);

        // e.storage()
        //     .instance()
        //     .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        // spend_allowance(&e, from.clone(), spender, amount);
        // spend_balance(&e, from.clone(), amount);
        // receive_balance(&e, to.clone(), amount);
        // _add_lender(e.clone(), to.clone());
        // TokenUtils::new(&e).events().transfer(from, to, amount)
    }

    fn burn(e: Env, from: Address, amount: i128) {
        // from.require_auth();
        // Admin only
        let admin = read_administrator(&e);
        admin.require_auth();

        check_nonnegative_amount(&e, amount);

        _burn(e, from, amount);
    }

    fn burn_from(e: Env, spender: Address, from: Address, amount: i128) {
        panic_with_error!(&e, Error::NotAllowed);
        // spender.require_auth();

        // check_nonnegative_amount(&e, amount);

        // spend_allowance(&e, from.clone(), spender, amount);
        // _burn(e, from, amount);
    }

    fn decimals(e: Env) -> u32 {
        read_decimal(&e)
    }

    fn name(e: Env) -> String {
        read_name(&e)
    }

    fn symbol(e: Env) -> String {
        read_symbol(&e)
    }
}
