use crate::error::Result;
use crate::ir::types::{GeneratedPolicy, PolicyParameters, PolicySpec};

pub fn emit(spec: &PolicySpec, package_name: &str) -> Result<GeneratedPolicy> {
    let (token, limit, window_ledgers) = match &spec.parameters {
        PolicyParameters::SpendingLimit { token, limit, window_ledgers } => {
            (token.as_str(), *limit, *window_ledgers)
        }
        _ => {
            return Err(crate::error::Error::Codegen(
                "spending_limit emitter called with non-SpendingLimit parameters".into(),
            ))
        }
    };

    let allowed_contracts: Vec<String> = spec
        .allowed_contracts
        .iter()
        .map(|c| format!("    // {c}"))
        .collect();
    let contracts_comment = allowed_contracts.join("\n");

    let cargo_toml = format!(
        r#"[package]
name    = "{package_name}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
soroban-sdk       = {{ version = "25.3.1" }}
stellar-contracts = {{ git = "https://github.com/OpenZeppelin/stellar-contracts", tag = "v0.1.0" }}
"#
    );

    let lib_rs = format!(
        r#"#![no_std]

// Configures the OpenZeppelin spending_limit policy primitive.
// Permitted contracts:
{contracts_comment}
//
// Token:          {token}
// Per-window cap: {limit}
// Window ledgers: {window_ledgers}
//
// Call install() with a SpendingLimitConfig to attach this policy to a context rule.
// The smart account enforces the cap automatically on every authorized transfer.

use soroban_sdk::{{contract, contractimpl, Address, Env, Symbol}};
use stellar_contracts::accounts::policy::spending_limit::{{SpendingLimitConfig, SpendingLimitPolicy}};

#[contract]
pub struct Policy;

#[contractimpl]
impl Policy {{
    pub fn install(env: Env, account: Address, rule_id: Symbol) {{
        let config = SpendingLimitConfig {{
            token: Address::from_str(&env, "{token}"),
            limit: {limit},
            window_ledgers: {window_ledgers},
        }};
        SpendingLimitPolicy::install(&env, &account, &rule_id, config);
    }}

    pub fn can_enforce(env: Env, account: Address, rule_id: Symbol) -> bool {{
        SpendingLimitPolicy::can_enforce(&env, &account, &rule_id)
    }}

    pub fn enforce(env: Env, account: Address, rule_id: Symbol, amount: i128) {{
        SpendingLimitPolicy::enforce(&env, &account, &rule_id, amount);
    }}

    pub fn uninstall(env: Env, account: Address, rule_id: Symbol) {{
        SpendingLimitPolicy::uninstall(&env, &account, &rule_id);
    }}
}}
"#
    );

    let tests_rs = format!(
        r#"#[cfg(test)]
mod tests {{
    use soroban_sdk::{{testutils::Address as _, Address, Env, Symbol}};
    use crate::PolicyClient;

    fn setup() -> (Env, Address, Symbol) {{
        let env = Env::default();
        let contract = env.register(crate::Policy, ());
        let account = Address::generate(&env);
        let rule_id = Symbol::new(&env, "{rule_id}");
        (env, account, rule_id)
    }}

    #[test]
    fn permit_within_limit() {{
        let (env, account, rule_id) = setup();
        let client = PolicyClient::new(&env, &env.register(crate::Policy, ()));
        client.install(&account, &rule_id);
        client.enforce(&account, &rule_id, &{limit});
    }}

    #[test]
    #[should_panic]
    fn deny_exceeds_limit() {{
        let (env, account, rule_id) = setup();
        let client = PolicyClient::new(&env, &env.register(crate::Policy, ()));
        client.install(&account, &rule_id);
        client.enforce(&account, &rule_id, &{over_limit});
    }}
}}
"#,
        rule_id = spec.context_rule_id,
        over_limit = limit.saturating_add(1),
    );

    Ok(GeneratedPolicy {
        cargo_toml,
        lib_rs,
        tests_rs,
        warnings: spec.warnings.clone(),
    })
}
