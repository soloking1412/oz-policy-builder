use crate::error::Result;
use crate::ir::types::{GeneratedPolicy, PolicyParameters, PolicySpec};

pub fn emit(spec: &PolicySpec, package_name: &str) -> Result<GeneratedPolicy> {
    let threshold = match &spec.parameters {
        PolicyParameters::SimpleThreshold { threshold } => *threshold,
        _ => {
            return Err(crate::error::Error::Codegen(
                "simple_threshold emitter called with wrong parameters".into(),
            ))
        }
    };

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

// Configures the OpenZeppelin simple_threshold policy primitive.
// Requires {threshold} valid signature(s) before authorizing any operation
// on the context rule "{rule_id}".

use soroban_sdk::{{contract, contractimpl, Address, Env, Symbol, Vec}};
use stellar_contracts::accounts::policy::simple_threshold::{{
    SimpleThresholdConfig, SimpleThresholdPolicy,
}};

#[contract]
pub struct Policy;

#[contractimpl]
impl Policy {{
    pub fn install(env: Env, account: Address, rule_id: Symbol, signers: Vec<Address>) {{
        let config = SimpleThresholdConfig {{
            signers,
            threshold: {threshold},
        }};
        SimpleThresholdPolicy::install(&env, &account, &rule_id, config);
    }}

    pub fn can_enforce(env: Env, account: Address, rule_id: Symbol) -> bool {{
        SimpleThresholdPolicy::can_enforce(&env, &account, &rule_id)
    }}

    pub fn enforce(env: Env, account: Address, rule_id: Symbol, signatures: Vec<soroban_sdk::Val>) {{
        SimpleThresholdPolicy::enforce(&env, &account, &rule_id, signatures);
    }}

    pub fn uninstall(env: Env, account: Address, rule_id: Symbol) {{
        SimpleThresholdPolicy::uninstall(&env, &account, &rule_id);
    }}
}}
"#,
        rule_id = spec.context_rule_id,
    );

    let tests_rs = format!(
        r#"#[cfg(test)]
mod tests {{
    use soroban_sdk::{{testutils::Address as _, Address, Env, Symbol, Vec}};

    #[test]
    #[ignore = "wire up signers before enabling"]
    fn permit_threshold_met() {{
        let env = Env::default();
        let _account = Address::generate(&env);
        let _rule_id = Symbol::new(&env, "{rule_id}");
    }}

    #[test]
    #[ignore = "wire up signers before enabling"]
    #[should_panic]
    fn deny_below_threshold() {{
        let env = Env::default();
        let _account = Address::generate(&env);
        let _rule_id = Symbol::new(&env, "{rule_id}");
    }}
}}
"#,
        rule_id = spec.context_rule_id,
    );

    Ok(GeneratedPolicy {
        cargo_toml,
        lib_rs,
        tests_rs,
        warnings: spec.warnings.clone(),
    })
}
