use crate::error::Result;
use crate::ir::types::{GeneratedPolicy, PolicyParameters, PolicySpec};

pub fn emit(spec: &PolicySpec, package_name: &str) -> Result<GeneratedPolicy> {
    let (signers, threshold) = match &spec.parameters {
        PolicyParameters::WeightedThreshold { signers, threshold } => (signers, *threshold),
        _ => {
            return Err(crate::error::Error::Codegen(
                "weighted_threshold emitter called with wrong parameters".into(),
            ))
        }
    };

    let signer_lines: Vec<String> = signers
        .iter()
        .map(|(addr, weight)| format!("        // {addr}  weight={weight}"))
        .collect();
    let signers_comment = signer_lines.join("\n");

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

// Configures the OpenZeppelin weighted_threshold policy primitive.
// Total threshold: {threshold}
// Signers and weights:
{signers_comment}

use soroban_sdk::{{contract, contractimpl, Address, Env, Symbol, Vec}};
use stellar_contracts::accounts::policy::weighted_threshold::{{
    WeightedSigner, WeightedThresholdConfig, WeightedThresholdPolicy,
}};

#[contract]
pub struct Policy;

#[contractimpl]
impl Policy {{
    pub fn install(env: Env, account: Address, rule_id: Symbol, signers: Vec<WeightedSigner>) {{
        let config = WeightedThresholdConfig {{
            signers,
            threshold: {threshold},
        }};
        WeightedThresholdPolicy::install(&env, &account, &rule_id, config);
    }}

    pub fn can_enforce(env: Env, account: Address, rule_id: Symbol) -> bool {{
        WeightedThresholdPolicy::can_enforce(&env, &account, &rule_id)
    }}

    pub fn enforce(env: Env, account: Address, rule_id: Symbol, signatures: Vec<soroban_sdk::Val>) {{
        WeightedThresholdPolicy::enforce(&env, &account, &rule_id, signatures);
    }}

    pub fn uninstall(env: Env, account: Address, rule_id: Symbol) {{
        WeightedThresholdPolicy::uninstall(&env, &account, &rule_id);
    }}
}}
"#
    );

    let tests_rs = format!(
        r#"#[cfg(test)]
mod tests {{
    use soroban_sdk::{{testutils::Address as _, Address, Env, Symbol}};

    #[test]
    #[ignore = "wire up signers and weights before enabling"]
    fn permit_weighted_threshold_met() {{
        let env = Env::default();
        let _account = Address::generate(&env);
        let _rule_id = Symbol::new(&env, "{rule_id}");
    }}

    #[test]
    #[ignore = "wire up signers and weights before enabling"]
    #[should_panic]
    fn deny_insufficient_weight() {{
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
