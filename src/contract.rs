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
use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};
use soroban_token_sdk::metadata::TokenMetadata;
use soroban_token_sdk::TokenUtils;

const PROTOCOL_FEE: i128 = 1000;
const REWARD_DENOM: i128 = 10000;

fn check_nonnegative_amount(amount: i128) {
    if amount < 0 {
        panic!("negative amount is not allowed: {}", amount)
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
        panic!("Start time not reached")
    }
}

fn require_final_time_not_reached(e: &Env) {
    let final_time: u64 = get_project_info(e).final_timestamp;
    if e.ledger().timestamp() > final_time {
        panic!("Final time reached")
    }
}

fn require_final_time_reached(e: &Env) {
    let final_time: u64 = get_project_info(e).final_timestamp;
    if e.ledger().timestamp() <= final_time {
        panic!("Final time not reached")
    }
}

fn require_target_amount_reached(e: &Env) {
    let target_amount: i128 = get_project_info(e).target_amount;
    if read_total_supply(&e) < target_amount {
        panic!("Target amount not reached")
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
        e.storage().persistent().set(
            &DataKey::LenderIndex(lender.clone()),
            &lender_index,
        );
        e.storage()
            .persistent()
            .set(&DataKey::LenderAddress(lender_index), &lender);
    }
}

#[contract]
pub struct Token;

fn move_token(env: &Env, from: &Address, to: &Address, transfer_amount: i128) {
    let token: Address = get_project_info(env).lend_token_address;
    // token interface
    let token_client: token::TokenClient<'_> = token::Client::new(&env, &token);
    token_client.transfer(&from, to, &transfer_amount);
}

#[contractimpl]
impl Token {
    pub fn initialize(
        e: Env,
        admin: Address,
        decimal: u32,
        name: String,
        symbol: String,
        project_info: ProjectInfo,
    ) {
        if has_administrator(&e) {
            panic!("already initialized")
        }
        write_administrator(&e, &admin);
        if decimal > u8::MAX.into() {
            panic!("Decimal must fit in a u8");
        }

        write_metadata(
            &e,
            TokenMetadata {
                decimal,
                name,
                symbol,
            },
        );

        let project_key: DataKey = DataKey::ProjectInfo;
        e.storage().persistent().set(&project_key, &project_info);
    }

    pub fn lend(e: Env, lender: Address, amount: i128) {
        check_nonnegative_amount(amount);
        lender.require_auth();

        require_start_time_reached(&e);
        require_final_time_not_reached(&e);

        let target_amount: i128 = get_project_info(&e).target_amount;
        let total_supply: i128 = read_total_supply(&e);
        if total_supply + amount > target_amount {
            panic!("Target amount overreached");
        }

        move_token(&e, &lender, &e.current_contract_address(), amount);
        _mint(e.clone(), lender.clone(), amount);
        _add_lender(e.clone(), lender.clone());
    }

    pub fn lender_claim(e: Env, lender: Address) {
        lender.require_auth();
        // TODO claimed balance
        let entitled_amount: i128 = 0;
        _burn(e.clone(), lender.clone(), entitled_amount);
        let reward_rate = get_project_info(&e).reward_rate;
        let amount_with_rewards = entitled_amount * reward_rate / REWARD_DENOM;
        move_token(&e, &e.current_contract_address(), &lender, amount_with_rewards);
    }

    pub fn borrower_claim(e: Env) {
        let borrower: Address = get_project_info(&e).borrower;
        borrower.require_auth();

        require_target_amount_reached(&e);
        require_final_time_reached(&e);

        let amount: i128 = read_total_supply(&e);
        move_token(&e, &e.current_contract_address(), &borrower, amount);
    }

    pub fn borrower_return(e: Env, borrower: Address, amount: i128) {
        borrower.require_auth();

        require_final_time_reached(&e);

        move_token(&e, &borrower, &e.current_contract_address(), amount);

        // Calculation of proportional return
        let total_supply: i128 = read_total_supply(&e);
        let number_of_lenders: u128 = read_number_of_lenders(&e);

        for i in 1..=number_of_lenders {
            let user_address: Address = e
                .storage()
                .persistent()
                .get(&DataKey::LenderAddress(i))
                .unwrap();
            let user_part: i128 = amount * read_balance(&e, user_address.clone()) / total_supply;
            move_token(&e, &e.current_contract_address(), &user_address, user_part);
        }
    }

    pub fn mint(e: Env, to: Address, amount: i128) {
        check_nonnegative_amount(amount);
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
impl token::Interface for Token {
    fn allowance(e: Env, from: Address, spender: Address) -> i128 {
        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        read_allowance(&e, from, spender).amount
    }

    fn approve(e: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
        panic!("Not allowed!");
        // from.require_auth();

        // check_nonnegative_amount(amount);

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
        panic!("Not allowed!");
        // from.require_auth();

        // check_nonnegative_amount(amount);

        // e.storage()
        //     .instance()
        //     .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        // spend_balance(&e, from.clone(), amount);
        // receive_balance(&e, to.clone(), amount);
        // _add_lender(e.clone(), to.clone());
        // TokenUtils::new(&e).events().transfer(from, to, amount);
    }

    fn transfer_from(e: Env, spender: Address, from: Address, to: Address, amount: i128) {
        panic!("Not allowed!");
        // spender.require_auth();

        // check_nonnegative_amount(amount);

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

        check_nonnegative_amount(amount);

        _burn(e, from, amount);
    }

    fn burn_from(e: Env, spender: Address, from: Address, amount: i128) {
        panic!("Not allowed!");
        // spender.require_auth();

        // check_nonnegative_amount(amount);

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
