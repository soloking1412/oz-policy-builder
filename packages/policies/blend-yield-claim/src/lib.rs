#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, panic_with_error, symbol_short, Address, Env, Symbol,
    Val, Vec,
};

mod error {
    use soroban_sdk::contracterror;

    #[contracterror]
    #[derive(Copy, Clone)]
    pub enum PolicyError {
        NotInstalled        = 1,
        ContractNotAllowed  = 2,
        FunctionNotAllowed  = 3,
        WindowLimitExceeded = 4,
    }
}

#[contracttype]
pub enum Key {
    Config(Address, Symbol),
    WindowStart(Address, Symbol),
    Accumulated(Address, Symbol),
}

#[contracttype]
pub struct Config {
    pub backstop: Address,
    pub pool: Address,
    pub blnd_token: Address,
    pub max_per_window: i128,
    pub window_ledgers: u32,
}

#[contract]
pub struct BlendYieldClaimPolicy;

#[contractimpl]
impl BlendYieldClaimPolicy {
    pub fn install(env: Env, account: Address, rule_id: Symbol, config: Config) {
        env.storage()
            .persistent()
            .set(&Key::Config(account.clone(), rule_id.clone()), &config);
        env.storage()
            .persistent()
            .set(&Key::WindowStart(account.clone(), rule_id.clone()), &env.ledger().sequence());
        env.storage()
            .persistent()
            .set(&Key::Accumulated(account, rule_id), &0i128);
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
        _args: Vec<Val>,
    ) {
        let config: Config = env
            .storage()
            .persistent()
            .get(&Key::Config(account.clone(), rule_id.clone()))
            .unwrap_or_else(|| panic_with_error!(&env, error::PolicyError::NotInstalled));

        if function != symbol_short!("claim") && function != symbol_short!("deposit") {
            panic_with_error!(&env, error::PolicyError::FunctionNotAllowed);
        }

        if target_contract != config.backstop && target_contract != config.pool {
            panic_with_error!(&env, error::PolicyError::ContractNotAllowed);
        }

        let window_start: u32 = env
            .storage()
            .persistent()
            .get(&Key::WindowStart(account.clone(), rule_id.clone()))
            .unwrap_or_else(|| env.ledger().sequence());

        let mut accumulated: i128 = env
            .storage()
            .persistent()
            .get(&Key::Accumulated(account.clone(), rule_id.clone()))
            .unwrap_or(0);

        if env.ledger().sequence() > window_start.saturating_add(config.window_ledgers) {
            accumulated = 0;
            env.storage().persistent().set(
                &Key::WindowStart(account.clone(), rule_id.clone()),
                &env.ledger().sequence(),
            );
        }

        if accumulated.saturating_add(1) > config.max_per_window {
            panic_with_error!(&env, error::PolicyError::WindowLimitExceeded);
        }

        env.storage()
            .persistent()
            .set(&Key::Accumulated(account, rule_id), &accumulated.saturating_add(1));
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
            .remove(&Key::Accumulated(account, rule_id));
    }
}

#[cfg(test)]
mod tests {
    use soroban_sdk::{
        testutils::Address as _, Address, Env, Symbol,
    };
    use super::{BlendYieldClaimPolicy, BlendYieldClaimPolicyClient, Config};

    fn setup() -> (Env, Address, Address, Address, Symbol) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(BlendYieldClaimPolicy, ());
        let backstop = Address::generate(&env);
        let pool = Address::generate(&env);
        let account = Address::generate(&env);
        let rule_id = Symbol::new(&env, "blend_yield");
        let client = BlendYieldClaimPolicyClient::new(&env, &contract_id);
        let config = Config {
            backstop: backstop.clone(),
            pool: pool.clone(),
            blnd_token: Address::generate(&env),
            max_per_window: 10,
            window_ledgers: 17_280,
        };
        client.install(&account, &rule_id, &config);
        (env, contract_id, backstop, account, rule_id)
    }

    #[test]
    fn permit_claim_on_backstop() {
        let (env, contract_id, backstop, account, rule_id) = setup();
        let client = BlendYieldClaimPolicyClient::new(&env, &contract_id);
        client.enforce(
            &account,
            &rule_id,
            &backstop,
            &soroban_sdk::symbol_short!("claim"),
            &soroban_sdk::vec![&env],
        );
    }

    #[test]
    #[should_panic]
    fn deny_unknown_contract() {
        let (env, contract_id, _, account, rule_id) = setup();
        let client = BlendYieldClaimPolicyClient::new(&env, &contract_id);
        client.enforce(
            &account,
            &rule_id,
            &Address::generate(&env),
            &soroban_sdk::symbol_short!("claim"),
            &soroban_sdk::vec![&env],
        );
    }

    #[test]
    #[should_panic]
    fn deny_unauthorized_function() {
        let (env, contract_id, backstop, account, rule_id) = setup();
        let client = BlendYieldClaimPolicyClient::new(&env, &contract_id);
        client.enforce(
            &account,
            &rule_id,
            &backstop,
            &soroban_sdk::symbol_short!("transfer"),
            &soroban_sdk::vec![&env],
        );
    }

    #[test]
    #[should_panic]
    fn deny_exceeds_window_limit() {
        let (env, contract_id, backstop, account, rule_id) = setup();
        let client = BlendYieldClaimPolicyClient::new(&env, &contract_id);
        for _ in 0..11 {
            client.enforce(
                &account,
                &rule_id,
                &backstop,
                &soroban_sdk::symbol_short!("claim"),
                &soroban_sdk::vec![&env],
            );
        }
    }
}
