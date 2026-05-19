use crate::error::Result;
use crate::ir::types::{
    HarnessReport, HarnessVector, Outcome, PolicySpec, VectorResult,
};
use crate::harness::generator;

pub fn run(spec: &PolicySpec, extra_permit: Vec<HarnessVector>, extra_deny: Vec<HarnessVector>) -> Result<HarnessReport> {
    let (mut permit_vecs, mut deny_vecs) = generator::generate(spec);
    permit_vecs.extend(extra_permit);
    deny_vecs.extend(extra_deny);

    let mut results: Vec<VectorResult> = Vec::new();

    for v in permit_vecs {
        results.push(evaluate_vector(&v, spec));
    }
    for v in deny_vecs {
        results.push(evaluate_vector(&v, spec));
    }

    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.len() - passed;

    Ok(HarnessReport { results, passed, failed })
}

fn evaluate_vector(vec: &HarnessVector, spec: &PolicySpec) -> VectorResult {
    let actual = simulate_enforcement(vec, spec);
    let passed = actual == vec.expected;
    let failure_reason = if passed {
        None
    } else {
        Some(format!(
            "expected {:?} but got {:?} for vector '{}'",
            vec.expected, actual, vec.id
        ))
    };

    VectorResult {
        vector_id: vec.id.clone(),
        expected: vec.expected.clone(),
        actual,
        passed,
        failure_reason,
    }
}

fn simulate_enforcement(vec: &HarnessVector, spec: &PolicySpec) -> Outcome {
    if !spec.allowed_contracts.contains(&vec.contract) {
        return Outcome::Deny;
    }

    let allowed_fn = spec.allowed_calls.iter().any(|c| c.function == vec.function);
    if !allowed_fn {
        return Outcome::Deny;
    }

    for call in &spec.allowed_calls {
        if call.function != vec.function {
            continue;
        }
        for constraint in &call.arg_constraints {
            if let Some(outcome) = check_constraint(constraint, &vec.args) {
                if outcome == Outcome::Deny {
                    return Outcome::Deny;
                }
            }
        }
    }

    Outcome::Permit
}

fn check_constraint(
    constraint: &crate::ir::types::ArgConstraint,
    args: &[crate::ir::types::ScValIR],
) -> Option<Outcome> {
    use crate::ir::types::{ArgConstraint, ScValIR};

    match constraint {
        ArgConstraint::MaxAmount { arg_index, max } => {
            let val = args.get(*arg_index)?;
            let amount = match val {
                ScValIR::I128(n) => *n,
                ScValIR::U128(n) => *n as i128,
                _ => return None,
            };
            if amount > *max {
                Some(Outcome::Deny)
            } else {
                Some(Outcome::Permit)
            }
        }

        ArgConstraint::ExactAddress { arg_index, address } => {
            let val = args.get(*arg_index)?;
            if let ScValIR::Address(a) = val {
                if a != address {
                    return Some(Outcome::Deny);
                }
            }
            Some(Outcome::Permit)
        }

        ArgConstraint::PathContains { arg_index, allowed } => {
            let val = args.get(*arg_index)?;
            if let ScValIR::Vec(items) = val {
                for item in items {
                    if let ScValIR::Address(a) = item {
                        if !allowed.contains(a) {
                            return Some(Outcome::Deny);
                        }
                    }
                }
            }
            Some(Outcome::Permit)
        }

        ArgConstraint::Deadline { arg_index } => {
            let val = args.get(*arg_index)?;
            if let ScValIR::U64(ts) = val {
                if *ts == 0 {
                    return Some(Outcome::Deny);
                }
            }
            Some(Outcome::Permit)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::types::{AllowedCall, ArgConstraint, PolicyKind, PolicyParameters, ScValIR};

    fn spec_with_transfer(contract: &str, max: i128) -> PolicySpec {
        PolicySpec {
            kind: PolicyKind::Custom,
            context_rule_id: "test".to_string(),
            allowed_contracts: vec![contract.to_string()],
            allowed_calls: vec![AllowedCall {
                contract: contract.to_string(),
                function: "transfer".to_string(),
                arg_constraints: vec![ArgConstraint::MaxAmount { arg_index: 0, max }],
            }],
            parameters: PolicyParameters::Custom,
            reasoning: vec![],
            warnings: vec![],
        }
    }

    #[test]
    fn all_auto_vectors_pass() {
        let spec = spec_with_transfer("CCONTRACT", 1000);
        let report = run(&spec, vec![], vec![]).unwrap();
        assert_eq!(report.failed, 0, "failed vectors: {:?}",
            report.results.iter().filter(|r| !r.passed).collect::<Vec<_>>());
    }

    #[test]
    fn custom_deny_vector_detected() {
        let spec = spec_with_transfer("CCONTRACT", 1000);
        let bad_vector = HarnessVector {
            id: "custom_deny".to_string(),
            expected: Outcome::Deny,
            contract: "CCONTRACT".to_string(),
            function: "transfer".to_string(),
            args: vec![ScValIR::I128(9999)],
        };
        let report = run(&spec, vec![], vec![bad_vector]).unwrap();
        assert_eq!(report.failed, 0);
    }
}
