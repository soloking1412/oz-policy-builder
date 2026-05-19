use crate::ir::types::{ArgConstraint, HarnessVector, Outcome, PolicySpec, ScValIR};

pub fn generate(spec: &PolicySpec) -> (Vec<HarnessVector>, Vec<HarnessVector>) {
    let mut permit = Vec::new();
    let mut deny = Vec::new();

    for call in &spec.allowed_calls {
        permit.push(HarnessVector {
            id: format!("permit_exact_{}", call.function),
            expected: Outcome::Permit,
            contract: call.contract.clone(),
            function: call.function.clone(),
            args: exact_args(call),
        });

        deny.push(HarnessVector {
            id: format!("deny_wrong_contract_{}", call.function),
            expected: Outcome::Deny,
            contract: "CBADCONTRACTADDRESS0000000000000000000000000000000000000000".to_string(),
            function: call.function.clone(),
            args: exact_args(call),
        });

        deny.push(HarnessVector {
            id: format!("deny_wrong_function_{}", call.function),
            expected: Outcome::Deny,
            contract: call.contract.clone(),
            function: "unauthorized_operation".to_string(),
            args: vec![],
        });

        for constraint in &call.arg_constraints {
            match constraint {
                ArgConstraint::MaxAmount { arg_index, max } => {
                    let mut args = exact_args(call);
                    if *arg_index < args.len() {
                        args[*arg_index] = ScValIR::I128(max.saturating_mul(2));
                    }
                    deny.push(HarnessVector {
                        id: format!("deny_amount_2x_{}", call.function),
                        expected: Outcome::Deny,
                        contract: call.contract.clone(),
                        function: call.function.clone(),
                        args,
                    });
                }

                ArgConstraint::ExactAddress { arg_index, .. } => {
                    let mut args = exact_args(call);
                    if *arg_index < args.len() {
                        args[*arg_index] =
                            ScValIR::Address("CWRONGADDRESS000000000000000000000000000000000000000000".to_string());
                    }
                    deny.push(HarnessVector {
                        id: format!("deny_address_mismatch_{}", call.function),
                        expected: Outcome::Deny,
                        contract: call.contract.clone(),
                        function: call.function.clone(),
                        args,
                    });
                }

                ArgConstraint::PathContains { arg_index, allowed } => {
                    let deviated_path = {
                        let mut p = allowed.clone();
                        p.push("CUNKNOWNTOKEN000000000000000000000000000000000000000000".to_string());
                        p
                    };
                    let mut args = exact_args(call);
                    if *arg_index < args.len() {
                        args[*arg_index] = ScValIR::Vec(
                            deviated_path
                                .into_iter()
                                .map(ScValIR::Address)
                                .collect(),
                        );
                    }
                    deny.push(HarnessVector {
                        id: format!("deny_path_deviation_{}", call.function),
                        expected: Outcome::Deny,
                        contract: call.contract.clone(),
                        function: call.function.clone(),
                        args,
                    });
                }

                ArgConstraint::Deadline { arg_index } => {
                    let mut args = exact_args(call);
                    if *arg_index < args.len() {
                        args[*arg_index] = ScValIR::U64(0);
                    }
                    deny.push(HarnessVector {
                        id: format!("deny_expired_deadline_{}", call.function),
                        expected: Outcome::Deny,
                        contract: call.contract.clone(),
                        function: call.function.clone(),
                        args,
                    });
                }
            }
        }
    }

    (permit, deny)
}

fn exact_args(call: &crate::ir::types::AllowedCall) -> Vec<ScValIR> {
    let max_idx = call.arg_constraints.iter().map(|c| match c {
        ArgConstraint::MaxAmount { arg_index, .. }
        | ArgConstraint::ExactAddress { arg_index, .. }
        | ArgConstraint::PathContains { arg_index, .. }
        | ArgConstraint::Deadline { arg_index } => *arg_index,
    }).max().unwrap_or(0);

    let mut args = vec![ScValIR::I128(0); max_idx + 1];

    for constraint in &call.arg_constraints {
        match constraint {
            ArgConstraint::MaxAmount { arg_index, max } => {
                args[*arg_index] = ScValIR::I128(*max / 2);
            }
            ArgConstraint::ExactAddress { arg_index, address } => {
                args[*arg_index] = ScValIR::Address(address.clone());
            }
            ArgConstraint::PathContains { arg_index, allowed } => {
                args[*arg_index] =
                    ScValIR::Vec(allowed.iter().cloned().map(ScValIR::Address).collect());
            }
            ArgConstraint::Deadline { arg_index } => {
                args[*arg_index] = ScValIR::U64(u64::MAX);
            }
        }
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::types::{AllowedCall, ArgConstraint};

    fn call_with_amount(contract: &str, function: &str, max: i128) -> AllowedCall {
        AllowedCall {
            contract: contract.to_string(),
            function: function.to_string(),
            arg_constraints: vec![ArgConstraint::MaxAmount { arg_index: 0, max }],
        }
    }

    #[test]
    fn generates_permit_and_deny_for_amount_constraint() {
        let spec = PolicySpec {
            kind: crate::ir::types::PolicyKind::Custom,
            context_rule_id: "test".to_string(),
            allowed_contracts: vec!["C1".to_string()],
            allowed_calls: vec![call_with_amount("C1", "transfer", 1000)],
            parameters: crate::ir::types::PolicyParameters::Custom,
            reasoning: vec![],
            warnings: vec![],
        };

        let (permit, deny) = generate(&spec);
        assert!(!permit.is_empty());
        assert!(deny.iter().any(|d| d.id.contains("deny_amount_2x")));
        assert!(deny.iter().any(|d| d.id.contains("deny_wrong_contract")));
        assert!(deny.iter().any(|d| d.id.contains("deny_wrong_function")));
    }
}
