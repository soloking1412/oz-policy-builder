#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, panic_with_error, Address, Env, Symbol,
    TryIntoVal, Val, Vec,
};

mod error {
    use soroban_sdk::contracterror;

    #[contracterror]
    #[derive(Copy, Clone)]
    pub enum PolicyError {
        NotInstalled        = 1,
        ContractNotAllowed  = 2,
        FunctionNotAllowed  = 3,
        AmountExceedsLimit  = 4,
        WindowLimitExceeded = 5,
        PathNotAllowed      = 6,
        DeadlineExpired     = 7,
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
    pub router: Address,
    pub allowed_tokens: Vec<Address>,
    pub max_amount_in: i128,
    pub window_ledgers: u32,
}

#[contract]
pub struct SoroswapBoundedPolicy;

#[contractimpl]
impl SoroswapBoundedPolicy {
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
        args: Vec<Val>,
    ) {
        let config: Config = env
            .storage()
            .persistent()
            .get(&Key::Config(account.clone(), rule_id.clone()))
            .unwrap_or_else(|| panic_with_error!(&env, error::PolicyError::NotInstalled));

        if target_contract != config.router {
            panic_with_error!(&env, error::PolicyError::ContractNotAllowed);
        }

        if function != Symbol::new(&env, "swap_exact_tokens_for_tokens")
            && function != Symbol::new(&env, "swap_tokens_for_exact_tokens")
        {
            panic_with_error!(&env, error::PolicyError::FunctionNotAllowed);
        }

        let amount_in: i128 = args
            .get(0)
            .as_ref()
            .and_then(|v| TryIntoVal::<_, i128>::try_into_val(v, &env).ok())
            .unwrap_or(0);

        if amount_in > config.max_amount_in {
            panic_with_error!(&env, error::PolicyError::AmountExceedsLimit);
        }

        let path: Vec<Address> = args
            .get(2)
            .as_ref()
            .and_then(|v| TryIntoVal::<_, Vec<Address>>::try_into_val(v, &env).ok())
            .unwrap_or_else(|| soroban_sdk::vec![&env]);

        for token in path.iter() {
            if !config.allowed_tokens.contains(&token) {
                panic_with_error!(&env, error::PolicyError::PathNotAllowed);
            }
        }

        let deadline: u64 = args
            .get(4)
            .as_ref()
            .and_then(|v| TryIntoVal::<_, u64>::try_into_val(v, &env).ok())
            .unwrap_or(0);

        if deadline < env.ledger().timestamp() {
            panic_with_error!(&env, error::PolicyError::DeadlineExpired);
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

        accumulated = accumulated.saturating_add(amount_in);
        if accumulated > config.max_amount_in {
            panic_with_error!(&env, error::PolicyError::WindowLimitExceeded);
        }

        env.storage()
            .persistent()
            .set(&Key::Accumulated(account, rule_id), &accumulated);
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
    use soroban_sdk::{testutils::{Address as _, Ledger}, vec, Address, Env, Symbol};
    use super::{Config, SoroswapBoundedPolicy, SoroswapBoundedPolicyClient};

    fn setup(max_amount_in: i128) -> (Env, Address, Address, Address, Address, Address, Symbol) {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|l| l.timestamp = 1_000_000);

        let contract_id = env.register(SoroswapBoundedPolicy, ());
        let router = Address::generate(&env);
        let usdc = Address::generate(&env);
        let xlm = Address::generate(&env);
        let account = Address::generate(&env);
        let rule_id = Symbol::new(&env, "soroswap_bounded");

        let config = Config {
            router: router.clone(),
            allowed_tokens: vec![&env, usdc.clone(), xlm.clone()],
            max_amount_in,
            window_ledgers: 17_280,
        };

        SoroswapBoundedPolicyClient::new(&env, &contract_id).install(&account, &rule_id, &config);
        (env, contract_id, router, usdc, xlm, account, rule_id)
    }

    fn make_swap_args(
        env: &Env,
        amount_in: i128,
        path: soroban_sdk::Vec<soroban_sdk::Val>,
        deadline: u64,
    ) -> soroban_sdk::Vec<soroban_sdk::Val> {
        use soroban_sdk::IntoVal;
        vec![
            env,
            amount_in.into_val(env),
            0i128.into_val(env),
            path.into_val(env),
            Address::generate(env).into_val(env),
            deadline.into_val(env),
        ]
    }

    #[test]
    fn permit_swap_within_bounds() {
        let (env, contract_id, router, usdc, xlm, account, rule_id) = setup(1_000_0000000);
        let client = SoroswapBoundedPolicyClient::new(&env, &contract_id);
        use soroban_sdk::IntoVal;
        let path: soroban_sdk::Vec<soroban_sdk::Val> = soroban_sdk::vec![
            &env,
            usdc.into_val(&env),
            xlm.into_val(&env),
        ];
        let args = make_swap_args(&env, 500_0000000i128, path, 2_000_000u64);
        client.enforce(
            &account,
            &rule_id,
            &router,
            &Symbol::new(&env, "swap_exact_tokens_for_tokens"),
            &args,
        );
    }

    #[test]
    #[should_panic]
    fn deny_exceeds_max_amount_in() {
        let (env, contract_id, router, _, _, account, rule_id) = setup(1_000_0000000);
        let client = SoroswapBoundedPolicyClient::new(&env, &contract_id);
        client.enforce(
            &account,
            &rule_id,
            &router,
            &Symbol::new(&env, "swap_exact_tokens_for_tokens"),
            &vec![
                &env,
                soroban_sdk::IntoVal::<soroban_sdk::Env, soroban_sdk::Val>::into_val(
                    &99_999_0000000i128, &env
                ),
            ],
        );
    }

    #[test]
    #[should_panic]
    fn deny_wrong_router() {
        let (env, contract_id, _, _, _, account, rule_id) = setup(1_000_0000000);
        let client = SoroswapBoundedPolicyClient::new(&env, &contract_id);
        client.enforce(
            &account,
            &rule_id,
            &Address::generate(&env),
            &Symbol::new(&env, "swap_exact_tokens_for_tokens"),
            &vec![&env],
        );
    }
}
