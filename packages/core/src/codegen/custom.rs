use crate::error::Result;
use crate::ir::types::{AllowedCall, ArgConstraint, GeneratedPolicy, PolicySpec};

pub fn emit(spec: &PolicySpec, package_name: &str) -> Result<GeneratedPolicy> {
    let cargo_toml = render_cargo_toml(package_name);
    let lib_rs = render_lib_rs(spec);
    let tests_rs = render_tests_rs(spec);

    Ok(GeneratedPolicy {
        cargo_toml,
        lib_rs,
        tests_rs,
        warnings: spec.warnings.clone(),
    })
}

fn render_cargo_toml(package_name: &str) -> String {
    format!(
        r#"[package]
name    = "{package_name}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
soroban-sdk = {{ version = "25.3.1" }}

[dev-dependencies]
soroban-sdk = {{ version = "25.3.1", features = ["testutils"] }}
"#
    )
}

fn render_lib_rs(spec: &PolicySpec) -> String {
    let contracts_array = render_string_array(&spec.allowed_contracts);
    let enforce_checks = render_enforce_checks(&spec.allowed_calls);
    let has_window = spec.allowed_calls.iter().any(|c| {
        c.arg_constraints
            .iter()
            .any(|a| matches!(a, ArgConstraint::MaxAmount { .. }))
    });

    let storage_extra = if has_window {
        "    WindowStart(Address, Symbol),\n    Accumulated(Address, Symbol),"
    } else {
        ""
    };

    let install_extra = if has_window {
        r#"
        env.storage()
            .persistent()
            .set(&Key::WindowStart(account.clone(), rule_id.clone()), &env.ledger().sequence());
        env.storage()
            .persistent()
            .set(&Key::Accumulated(account, rule_id), &0i128);"#
    } else {
        ""
    };

    let uninstall_extra = if has_window {
        r#"
        env.storage()
            .persistent()
            .remove(&Key::WindowStart(account.clone(), rule_id.clone()));
        env.storage()
            .persistent()
            .remove(&Key::Accumulated(account.clone(), rule_id.clone()));"#
    } else {
        ""
    };

    let window_ledgers = extract_window_ledgers(spec);

    format!(
        r#"#![no_std]

use soroban_sdk::{{
    contract, contractimpl, contracttype, panic_with_error, vec, Address, Env, Symbol, Val, Vec,
}};

mod error {{
    use soroban_sdk::contracterror;

    #[contracterror]
    #[derive(Copy, Clone)]
    pub enum PolicyError {{
        PolicyNotInstalled  = 1,
        ContractNotAllowed  = 2,
        FunctionNotAllowed  = 3,
        AmountExceedsLimit  = 4,
        WindowLimitExceeded = 5,
        AddressMismatch     = 6,
        PathNotAllowed      = 7,
        DeadlineExpired     = 8,
    }}
}}

#[contracttype]
pub enum Key {{
    Config(Address, Symbol),
{storage_extra}
}}

#[contracttype]
pub struct Config {{
    pub window_ledgers: u32,
    pub max_amount: i128,
}}

const ALLOWED_CONTRACTS: &[&str] = &{contracts_array};

#[contract]
pub struct Policy;

#[contractimpl]
impl Policy {{
    pub fn install(env: Env, account: Address, rule_id: Symbol, max_amount: i128, window_ledgers: u32) {{
        let config = Config {{ window_ledgers, max_amount }};
        env.storage()
            .persistent()
            .set(&Key::Config(account.clone(), rule_id.clone()), &config);{install_extra}
    }}

    pub fn can_enforce(_env: Env, _account: Address, _rule_id: Symbol) -> bool {{
        true
    }}

    pub fn enforce(
        env: Env,
        account: Address,
        rule_id: Symbol,
        target_contract: Address,
        function: Symbol,
        args: Vec<Val>,
    ) {{
        let config: Config = env
            .storage()
            .persistent()
            .get(&Key::Config(account.clone(), rule_id.clone()))
            .unwrap_or_else(|| panic_with_error!(&env, error::PolicyError::PolicyNotInstalled));

        Self::check_contract(&env, &target_contract);
        Self::check_function(&env, &function);
{enforce_checks}
{window_check}
    }}

    pub fn uninstall(env: Env, account: Address, rule_id: Symbol) {{
        env.storage()
            .persistent()
            .remove(&Key::Config(account.clone(), rule_id.clone()));{uninstall_extra}
    }}

    fn check_contract(env: &Env, contract: &Address) {{
        let s = contract.to_string();
        if !ALLOWED_CONTRACTS.iter().any(|a| *a == s.as_str()) {{
            panic_with_error!(env, error::PolicyError::ContractNotAllowed);
        }}
    }}

    fn check_function(env: &Env, function: &Symbol) {{
        let allowed_fns: &[&str] = &[{allowed_fns}];
        let fn_str = function.to_string();
        if !allowed_fns.iter().any(|f| *f == fn_str.as_str()) {{
            panic_with_error!(env, error::PolicyError::FunctionNotAllowed);
        }}
    }}
}}
"#,
        window_check = render_window_check(spec, window_ledgers),
        allowed_fns = render_function_list(spec),
    )
}

fn render_enforce_checks(calls: &[AllowedCall]) -> String {
    let mut lines: Vec<String> = Vec::new();

    for call in calls {
        for constraint in &call.arg_constraints {
            match constraint {
                ArgConstraint::MaxAmount { arg_index, max } => {
                    lines.push(format!(
                        r#"        if let Some(amount) = extract_i128(&args, {arg_index}) {{
            if amount > {max} {{
                panic_with_error!(&env, error::PolicyError::AmountExceedsLimit);
            }}
        }}"#
                    ));
                }
                ArgConstraint::ExactAddress { arg_index, address } => {
                    lines.push(format!(
                        r#"        if let Some(addr) = extract_address(&args, {arg_index}) {{
            if addr.to_string() != "{address}" {{
                panic_with_error!(&env, error::PolicyError::AddressMismatch);
            }}
        }}"#
                    ));
                }
                ArgConstraint::PathContains { arg_index, allowed } => {
                    let allowed_list = allowed
                        .iter()
                        .map(|a| format!("\"{a}\""))
                        .collect::<Vec<_>>()
                        .join(", ");
                    lines.push(format!(
                        r#"        check_path(&env, &args, {arg_index}, &[{allowed_list}]);"#
                    ));
                }
                ArgConstraint::Deadline { arg_index } => {
                    lines.push(format!(
                        r#"        if let Some(deadline) = extract_u64(&args, {arg_index}) {{
            if deadline < env.ledger().timestamp() {{
                panic_with_error!(&env, error::PolicyError::DeadlineExpired);
            }}
        }}"#
                    ));
                }
            }
        }
    }

    lines.join("\n")
}

fn render_window_check(spec: &PolicySpec, window_ledgers: u32) -> String {
    let has_amount = spec.allowed_calls.iter().any(|c| {
        c.arg_constraints
            .iter()
            .any(|a| matches!(a, ArgConstraint::MaxAmount { .. }))
    });

    if !has_amount {
        return String::new();
    }

    format!(
        r#"
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

        if env.ledger().sequence() > window_start.saturating_add({window_ledgers}) {{
            accumulated = 0;
            env.storage()
                .persistent()
                .set(&Key::WindowStart(account.clone(), rule_id.clone()), &env.ledger().sequence());
        }}

        if let Some(amount) = extract_i128(&args, 0) {{
            accumulated = accumulated.saturating_add(amount);
            if accumulated > config.max_amount {{
                panic_with_error!(&env, error::PolicyError::WindowLimitExceeded);
            }}
            env.storage()
                .persistent()
                .set(&Key::Accumulated(account, rule_id), &accumulated);
        }}"#
    )
}

fn extract_window_ledgers(spec: &PolicySpec) -> u32 {
    if let crate::ir::types::PolicyParameters::SpendingLimit { window_ledgers, .. } =
        &spec.parameters
    {
        return *window_ledgers;
    }
    17_280
}

fn render_string_array(items: &[String]) -> String {
    let inner: Vec<String> = items.iter().map(|s| format!("\"{s}\"")).collect();
    format!("[{}]", inner.join(", "))
}

fn render_function_list(spec: &PolicySpec) -> String {
    let mut seen = std::collections::BTreeSet::new();
    let fns: Vec<String> = spec
        .allowed_calls
        .iter()
        .filter_map(|c| seen.insert(c.function.clone()).then(|| format!("\"{}\"", c.function)))
        .collect();
    fns.join(", ")
}

fn render_tests_rs(spec: &PolicySpec) -> String {
    let max = spec
        .allowed_calls
        .iter()
        .flat_map(|c| c.arg_constraints.iter())
        .filter_map(|a| {
            if let ArgConstraint::MaxAmount { max, .. } = a {
                Some(*max)
            } else {
                None
            }
        })
        .max()
        .unwrap_or(1000);

    format!(
        r#"#[cfg(test)]
mod tests {{
    use soroban_sdk::{{
        testutils::{{Address as _, Ledger}},
        Address, Env, Symbol, vec,
    }};
    use crate::PolicyClient;

    fn setup() -> (Env, Address, Address, Symbol) {{
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::Policy, ());
        let account = Address::generate(&env);
        let rule_id = Symbol::new(&env, "{rule_id}");
        (env, contract_id, account, rule_id)
    }}

    #[test]
    fn permit_exact_amount() {{
        let (env, contract_id, account, rule_id) = setup();
        let client = PolicyClient::new(&env, &contract_id);
        client.install(&account, &rule_id, &{max}, &17280u32);
        client.enforce(
            &account,
            &rule_id,
            &Address::generate(&env),
            &Symbol::new(&env, "{first_fn}"),
            &vec![&env],
        );
    }}

    #[test]
    #[should_panic(expected = "AmountExceedsLimit")]
    fn deny_exceeds_limit() {{
        let (env, contract_id, account, rule_id) = setup();
        let client = PolicyClient::new(&env, &contract_id);
        client.install(&account, &rule_id, &{max}, &17280u32);
        client.enforce(
            &account,
            &rule_id,
            &Address::generate(&env),
            &Symbol::new(&env, "{first_fn}"),
            &vec![&env, &soroban_sdk::Val::from({over_limit}i128)],
        );
    }}

    #[test]
    #[should_panic(expected = "ContractNotAllowed")]
    fn deny_wrong_contract() {{
        let (env, contract_id, account, rule_id) = setup();
        let client = PolicyClient::new(&env, &contract_id);
        client.install(&account, &rule_id, &{max}, &17280u32);
        client.enforce(
            &account,
            &rule_id,
            &Address::generate(&env),
            &Symbol::new(&env, "{first_fn}"),
            &vec![&env],
        );
    }}

    #[test]
    #[should_panic(expected = "FunctionNotAllowed")]
    fn deny_wrong_function() {{
        let (env, contract_id, account, rule_id) = setup();
        let client = PolicyClient::new(&env, &contract_id);
        client.install(&account, &rule_id, &{max}, &17280u32);
        client.enforce(
            &account,
            &rule_id,
            &Address::generate(&env),
            &Symbol::new(&env, "unauthorized_fn"),
            &vec![&env],
        );
    }}
}}
"#,
        rule_id = spec.context_rule_id,
        first_fn = spec
            .allowed_calls
            .first()
            .map(|c| c.function.as_str())
            .unwrap_or("transfer"),
        over_limit = max.saturating_add(1),
    )
}
