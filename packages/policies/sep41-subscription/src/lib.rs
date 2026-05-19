#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, panic_with_error, symbol_short, Address, Env, Symbol,
    TryIntoVal, Val, Vec,
};

mod error {
    use soroban_sdk::contracterror;

    #[contracterror]
    #[derive(Copy, Clone)]
    pub enum PolicyError {
        NotInstalled         = 1,
        ContractNotAllowed   = 2,
        FunctionNotAllowed   = 3,
        AllowanceTooHigh     = 4,
        AmountExceedsPerCall = 5,
        WrongSpender         = 6,
        WrongBeneficiary     = 7,
        CallCountExceeded    = 8,
    }
}

#[contracttype]
pub enum Key {
    Config(Address, Symbol),
    CallCount(Address, Symbol),
    WindowStart(Address, Symbol),
}

#[contracttype]
pub struct Config {
    pub token: Address,
    pub subscriber: Address,
    pub beneficiary: Address,
    pub allowance_ceiling: i128,
    pub max_per_call: i128,
    pub max_calls_per_window: u32,
    pub window_ledgers: u32,
}

#[contract]
pub struct Sep41SubscriptionPolicy;

#[contractimpl]
impl Sep41SubscriptionPolicy {
    pub fn install(env: Env, account: Address, rule_id: Symbol, config: Config) {
        env.storage()
            .persistent()
            .set(&Key::Config(account.clone(), rule_id.clone()), &config);
        env.storage()
            .persistent()
            .set(&Key::WindowStart(account.clone(), rule_id.clone()), &env.ledger().sequence());
        env.storage()
            .persistent()
            .set(&Key::CallCount(account, rule_id), &0u32);
    }

    pub fn can_enforce(_env: Env, _account: Address, _rule_id: Symbol) -> bool {
        true
    }

    pub fn enforce(
        env: Env,
        account: Address,
        rule_id: Symbol,
        target_contract: Address,
        function: Symbol,
        args: Vec<Val>,
    ) {
        let config: Config = env
            .storage()
            .persistent()
            .get(&Key::Config(account.clone(), rule_id.clone()))
            .unwrap_or_else(|| panic_with_error!(&env, error::PolicyError::NotInstalled));

        if target_contract != config.token {
            panic_with_error!(&env, error::PolicyError::ContractNotAllowed);
        }

        if function == symbol_short!("approve") {
            Self::enforce_approve(&env, &config, &args);
        } else if function == Symbol::new(&env, "transfer_from") {
            Self::enforce_transfer_from(&env, &account, &rule_id, &config, &args);
        } else {
            panic_with_error!(&env, error::PolicyError::FunctionNotAllowed);
        }
    }

    pub fn uninstall(env: Env, account: Address, rule_id: Symbol) {
        env.storage()
            .persistent()
            .remove(&Key::Config(account.clone(), rule_id.clone()));
        env.storage()
            .persistent()
            .remove(&Key::WindowStart(account.clone(), rule_id.clone()));
        env.storage()
            .persistent()
            .remove(&Key::CallCount(account, rule_id));
    }

    fn enforce_approve(env: &Env, config: &Config, args: &Vec<Val>) {
        let spender: Address = args
            .get(1)
            .as_ref()
            .and_then(|v| TryIntoVal::<_, Address>::try_into_val(v, env).ok())
            .unwrap_or_else(|| panic_with_error!(env, error::PolicyError::WrongSpender));

        if spender != config.subscriber {
            panic_with_error!(env, error::PolicyError::WrongSpender);
        }

        let amount: i128 = args
            .get(2)
            .as_ref()
            .and_then(|v| TryIntoVal::<_, i128>::try_into_val(v, env).ok())
            .unwrap_or(0);

        if amount > config.allowance_ceiling {
            panic_with_error!(env, error::PolicyError::AllowanceTooHigh);
        }
    }

    fn enforce_transfer_from(
        env: &Env,
        account: &Address,
        rule_id: &Symbol,
        config: &Config,
        args: &Vec<Val>,
    ) {
        let amount: i128 = args
            .get(3)
            .as_ref()
            .and_then(|v| TryIntoVal::<_, i128>::try_into_val(v, env).ok())
            .unwrap_or(0);

        if amount > config.max_per_call {
            panic_with_error!(env, error::PolicyError::AmountExceedsPerCall);
        }

        let to: Address = args
            .get(2)
            .as_ref()
            .and_then(|v| TryIntoVal::<_, Address>::try_into_val(v, env).ok())
            .unwrap_or_else(|| panic_with_error!(env, error::PolicyError::WrongBeneficiary));

        if to != config.beneficiary {
            panic_with_error!(env, error::PolicyError::WrongBeneficiary);
        }

        let window_start: u32 = env
            .storage()
            .persistent()
            .get(&Key::WindowStart(account.clone(), rule_id.clone()))
            .unwrap_or_else(|| env.ledger().sequence());

        let mut call_count: u32 = env
            .storage()
            .persistent()
            .get(&Key::CallCount(account.clone(), rule_id.clone()))
            .unwrap_or(0);

        if env.ledger().sequence() > window_start.saturating_add(config.window_ledgers) {
            call_count = 0;
            env.storage().persistent().set(
                &Key::WindowStart(account.clone(), rule_id.clone()),
                &env.ledger().sequence(),
            );
        }

        call_count = call_count.saturating_add(1);
        if call_count > config.max_calls_per_window {
            panic_with_error!(env, error::PolicyError::CallCountExceeded);
        }

        env.storage()
            .persistent()
            .set(&Key::CallCount(account.clone(), rule_id.clone()), &call_count);
    }
}

#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::Address as _, IntoVal, Val, vec, Address, Env, Symbol};
    use super::{Config, Sep41SubscriptionPolicy, Sep41SubscriptionPolicyClient};

    fn setup() -> (Env, Address, Address, Address, Address, Symbol) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(Sep41SubscriptionPolicy, ());
        let token = Address::generate(&env);
        let subscriber = Address::generate(&env);
        let beneficiary = Address::generate(&env);
        let account = Address::generate(&env);
        let rule_id = Symbol::new(&env, "subscription");
        let config = Config {
            token: token.clone(),
            subscriber: subscriber.clone(),
            beneficiary: beneficiary.clone(),
            allowance_ceiling: 10_000,
            max_per_call: 1_000,
            max_calls_per_window: 30,
            window_ledgers: 17_280,
        };
        Sep41SubscriptionPolicyClient::new(&env, &contract_id).install(&account, &rule_id, &config);
        (env, contract_id, token, subscriber, account, rule_id)
    }

    #[test]
    fn permit_approve_within_ceiling() {
        let (env, contract_id, token, subscriber, account, rule_id) = setup();
        let client = Sep41SubscriptionPolicyClient::new(&env, &contract_id);
        let from = Address::generate(&env);
        client.enforce(
            &account,
            &rule_id,
            &token,
            &soroban_sdk::symbol_short!("approve"),
            &vec![
                &env,
                from.into_val(&env),
                subscriber.into_val(&env),
                IntoVal::<Env, Val>::into_val(&500i128, &env),
            ],
        );
    }

    #[test]
    #[should_panic]
    fn deny_approve_wrong_spender() {
        let (env, contract_id, token, _, account, rule_id) = setup();
        let client = Sep41SubscriptionPolicyClient::new(&env, &contract_id);
        let from = Address::generate(&env);
        let rogue = Address::generate(&env);
        client.enforce(
            &account,
            &rule_id,
            &token,
            &soroban_sdk::symbol_short!("approve"),
            &vec![
                &env,
                from.into_val(&env),
                rogue.into_val(&env),
                IntoVal::<Env, Val>::into_val(&500i128, &env),
            ],
        );
    }

    #[test]
    #[should_panic]
    fn deny_approve_exceeds_ceiling() {
        let (env, contract_id, token, subscriber, account, rule_id) = setup();
        let client = Sep41SubscriptionPolicyClient::new(&env, &contract_id);
        let from = Address::generate(&env);
        client.enforce(
            &account,
            &rule_id,
            &token,
            &soroban_sdk::symbol_short!("approve"),
            &vec![
                &env,
                from.into_val(&env),
                subscriber.into_val(&env),
                IntoVal::<Env, Val>::into_val(&99_999i128, &env),
            ],
        );
    }
}
