//! EnerDAO tokenized funding
use crate::admin::{has_administrator, read_administrator, write_administrator};
use crate::allowance::{read_allowance, spend_allowance, write_allowance};
use crate::balance::{read_balance, receive_balance, spend_balance};
use crate::metadata::{read_decimal, read_name, read_symbol, write_metadata};
#[cfg(test)]
use crate::storage_types::{AllowanceDataKey, AllowanceValue};
use crate::storage_types::{
    DataKey, BALANCE_BUMP_AMOUNT, BALANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT,
    INSTANCE_LIFETIME_THRESHOLD,
};
use soroban_sdk::token::{self, Interface as _};
use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};
use soroban_token_sdk::metadata::TokenMetadata;
use soroban_token_sdk::TokenUtils;

fn check_nonnegative_amount(amount: i128) {
    if amount < 0 {
        panic!("negative amount is not allowed: {}", amount)
    }
}

fn require_target_time_not_reached(e: &Env) {
    let key = DataKey::TargetTimestamp;
    let target_time: u64 = e.storage().persistent().get(&key).unwrap();
    if e.ledger().timestamp() > target_time {
        panic!("Target time reached")
    }
}

fn require_target_time_reached(e: &Env) {
    let key = DataKey::TargetTimestamp;
    let target_time: u64 = e.storage().persistent().get(&key).unwrap();
    if e.ledger().timestamp() <= target_time {
        panic!("Target time not reached")
    }
}

fn require_target_amount_reached(e: &Env) {
    let key = DataKey::TargetAmount;
    let target_amount: i128 = e.storage().persistent().get(&key).unwrap();
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

fn read_number_of_depositors(e: &Env) -> u128 {
    let key = DataKey::NumberOfDepositors;
    let number_of_depositors: u128 = e.storage().persistent().get(&key).unwrap_or(0);
    number_of_depositors
}

fn write_number_of_depositors(e: &Env, val: u128) {
    let key = DataKey::NumberOfDepositors;
    e.storage().persistent().set(&key, &val);
    e.storage()
        .persistent()
        .extend_ttl(&key, BALANCE_LIFETIME_THRESHOLD, BALANCE_BUMP_AMOUNT);
}

// internal function that records index of the depositor
// if this is a new depositor
fn _add_depositor(e: Env, depositor: Address) {
    let mut number_of_depositors: u128 = read_number_of_depositors(&e);
    let mut depositor_index: u128 = e
        .storage()
        .persistent()
        .get(&DataKey::DepositorIndex(depositor.clone()))
        .unwrap_or(0);

    if depositor_index == 0 {
        number_of_depositors += 1;
        depositor_index = number_of_depositors;
        write_number_of_depositors(&e, number_of_depositors);
        e.storage().persistent().set(
            &DataKey::DepositorIndex(depositor.clone()),
            &depositor_index,
        );
        e.storage()
            .persistent()
            .set(&DataKey::DepositorAddress(depositor_index), &depositor);
    }
}

#[contract]
pub struct Token;

fn move_token(env: &Env, from: &Address, to: &Address, transfer_amount: i128) {
    let token_key = DataKey::DepositTokenAddress;
    let token = env.storage().persistent().get(&token_key).unwrap();

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
        beneficiary: Address,
        deposit_token: Address,
        target_amount: i128,
        target_timestamp: u64,
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

        let beneficiary_key = DataKey::Beneficiary;
        e.storage().persistent().set(&beneficiary_key, &beneficiary);

        let token_key = DataKey::DepositTokenAddress;
        e.storage().persistent().set(&token_key, &deposit_token);

        let target_amount_key = DataKey::TargetAmount;
        e.storage()
            .persistent()
            .set(&target_amount_key, &target_amount);

        let target_timestamp_key = DataKey::TargetTimestamp;
        e.storage()
            .persistent()
            .set(&target_timestamp_key, &target_timestamp);
    }

    pub fn deposit(e: Env, depositor: Address, amount: i128) {
        check_nonnegative_amount(amount);
        depositor.require_auth();

        require_target_time_not_reached(&e);

        move_token(&e, &depositor, &e.current_contract_address(), amount);
        _mint(e.clone(), depositor.clone(), amount);
        _add_depositor(e.clone(), depositor.clone());
    }

    pub fn withdraw(e: Env, depositor: Address, amount: i128) {
        check_nonnegative_amount(amount);
        depositor.require_auth();

        _burn(e.clone(), depositor.clone(), amount);
        move_token(&e, &e.current_contract_address(), &depositor, amount);
    }

    pub fn beneficiary_claim(e: Env) {
        let beneficiary_key = DataKey::Beneficiary;
        let beneficiary: Address = e.storage().persistent().get(&beneficiary_key).unwrap();
        beneficiary.require_auth();

        require_target_amount_reached(&e);
        require_target_time_reached(&e);

        let amount: i128 = read_total_supply(&e);
        move_token(&e, &e.current_contract_address(), &beneficiary, amount);
    }

    pub fn beneficiary_return(e: Env, beneficiary: Address, amount: i128) {
        beneficiary.require_auth();

        require_target_time_reached(&e);

        move_token(&e, &beneficiary, &e.current_contract_address(), amount);

        // Calculation of proportional return
        let total_supply: i128 = read_total_supply(&e);
        let number_of_depositors: u128 = read_number_of_depositors(&e);

        for i in 1..=number_of_depositors {
            let user_address: Address = e
                .storage()
                .persistent()
                .get(&DataKey::DepositorAddress(i))
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

    pub fn number_of_depositors(e: Env) -> u128 {
        read_number_of_depositors(&e)
    }

    pub fn get_depositors(e: Env) -> Vec<Address> {
        let mut depositors: Vec<Address> = Vec::<Address>::new(&e);
        for i in 1..=read_number_of_depositors(&e) {
            let user_address: Address = e
                .storage()
                .persistent()
                .get(&DataKey::DepositorAddress(i))
                .unwrap();
            depositors.push_back(user_address);
        }
        depositors
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
        from.require_auth();

        check_nonnegative_amount(amount);

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        write_allowance(&e, from.clone(), spender.clone(), amount, expiration_ledger);
        TokenUtils::new(&e)
            .events()
            .approve(from, spender, amount, expiration_ledger);
    }

    fn balance(e: Env, id: Address) -> i128 {
        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        read_balance(&e, id)
    }

    fn transfer(e: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();

        check_nonnegative_amount(amount);

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        spend_balance(&e, from.clone(), amount);
        receive_balance(&e, to.clone(), amount);
        _add_depositor(e.clone(), to.clone());
        TokenUtils::new(&e).events().transfer(from, to, amount);
    }

    fn transfer_from(e: Env, spender: Address, from: Address, to: Address, amount: i128) {
        spender.require_auth();

        check_nonnegative_amount(amount);

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        spend_allowance(&e, from.clone(), spender, amount);
        spend_balance(&e, from.clone(), amount);
        receive_balance(&e, to.clone(), amount);
        _add_depositor(e.clone(), to.clone());
        TokenUtils::new(&e).events().transfer(from, to, amount)
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
        spender.require_auth();

        check_nonnegative_amount(amount);

        spend_allowance(&e, from.clone(), spender, amount);
        _burn(e, from, amount);
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
