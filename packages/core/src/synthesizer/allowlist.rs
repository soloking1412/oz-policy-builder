use crate::ir::types::{AllowedCall, ArgConstraint, AuthNode, ScValIR};

const AMOUNT_BEARING_FNS: &[&str] = &[
    "transfer",
    "transfer_from",
    "approve",
    "burn",
    "burn_from",
    "claim",
    "deposit",
    "withdraw",
    "submit",
    "swap_exact_tokens_for_tokens",
    "swap_tokens_for_exact_tokens",
];

pub fn build(nodes: &[&AuthNode], source: &str, multiplier: f64) -> Vec<AllowedCall> {
    nodes
        .iter()
        .map(|n| AllowedCall {
            contract: n.contract.clone(),
            function: n.function.clone(),
            arg_constraints: extract_constraints(n, source, multiplier),
        })
        .collect()
}

fn extract_constraints(node: &AuthNode, source: &str, multiplier: f64) -> Vec<ArgConstraint> {
    let mut constraints = Vec::new();

    for (idx, arg) in node.args.iter().enumerate() {
        match arg {
            ScValIR::I128(_) | ScValIR::U128(_)
                if AMOUNT_BEARING_FNS.contains(&node.function.as_str()) =>
            {
                let raw: i128 = match arg {
                    ScValIR::I128(n) => *n,
                    ScValIR::U128(n) => *n as i128,
                    _ => unreachable!(),
                };
                let ceiling = (raw as f64 * multiplier).ceil() as i128;
                constraints.push(ArgConstraint::MaxAmount { arg_index: idx, max: ceiling });
            }

            ScValIR::Address(a) if a != source => {
                constraints.push(ArgConstraint::ExactAddress {
                    arg_index: idx,
                    address: a.clone(),
                });
            }

            ScValIR::Vec(items)
                if node.function == "swap_exact_tokens_for_tokens"
                    || node.function == "swap_tokens_for_exact_tokens" =>
            {
                let addrs: Vec<String> = items
                    .iter()
                    .filter_map(|v| {
                        if let ScValIR::Address(a) = v {
                            Some(a.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                if !addrs.is_empty() {
                    constraints.push(ArgConstraint::PathContains {
                        arg_index: idx,
                        allowed: addrs,
                    });
                }
            }

            ScValIR::U64(_)
                if (node.function == "swap_exact_tokens_for_tokens"
                    || node.function == "swap_tokens_for_exact_tokens")
                    && idx == 4 =>
            {
                constraints.push(ArgConstraint::Deadline { arg_index: idx });
            }

            _ => {}
        }
    }

    constraints
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_with_args(function: &str, args: Vec<ScValIR>) -> AuthNode {
        AuthNode {
            signer: "S".to_string(),
            contract: "C".to_string(),
            function: function.to_string(),
            args,
            sub_invocations: vec![],
        }
    }

    #[test]
    fn extracts_max_amount_for_transfer() {
        let n = node_with_args("transfer", vec![
            ScValIR::Address("FROM".to_string()),
            ScValIR::Address("TO".to_string()),
            ScValIR::I128(1000),
        ]);
        let calls = build(&[&n], "FROM", 1.1);
        let constraints = &calls[0].arg_constraints;
        assert!(constraints
            .iter()
            .any(|c| matches!(c, ArgConstraint::MaxAmount { max, .. } if *max == 1100)));
    }

    #[test]
    fn extracts_exact_address_for_non_source() {
        let n = node_with_args("transfer", vec![
            ScValIR::Address("SOURCE".to_string()),
            ScValIR::Address("RECIPIENT".to_string()),
            ScValIR::I128(100),
        ]);
        let calls = build(&[&n], "SOURCE", 1.0);
        let constraints = &calls[0].arg_constraints;
        assert!(constraints.iter().any(|c| matches!(
            c,
            ArgConstraint::ExactAddress { address, .. } if address == "RECIPIENT"
        )));
        assert!(!constraints.iter().any(|c| matches!(
            c,
            ArgConstraint::ExactAddress { address, .. } if address == "SOURCE"
        )));
    }

    #[test]
    fn extracts_path_for_swap() {
        let path = ScValIR::Vec(vec![
            ScValIR::Address("USDC".to_string()),
            ScValIR::Address("XLM".to_string()),
        ]);
        let n = node_with_args("swap_exact_tokens_for_tokens", vec![
            ScValIR::I128(1000),
            ScValIR::I128(900),
            path,
            ScValIR::Address("DEST".to_string()),
            ScValIR::U64(99999),
        ]);
        let calls = build(&[&n], "DEST", 1.1);
        let constraints = &calls[0].arg_constraints;
        assert!(constraints.iter().any(|c| matches!(c, ArgConstraint::PathContains { .. })));
        assert!(constraints.iter().any(|c| matches!(c, ArgConstraint::Deadline { .. })));
    }
}
