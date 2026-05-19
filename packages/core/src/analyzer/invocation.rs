use crate::ir::types::{AuthNode, ProtocolHint, ScValIR};

const BLEND_FUNCTIONS: &[&str] = &["claim", "deposit", "withdraw", "submit", "gulp_emissions"];
const SOROSWAP_FUNCTIONS: &[&str] = &[
    "swap_exact_tokens_for_tokens",
    "swap_tokens_for_exact_tokens",
];
const SEP41_FUNCTIONS: &[&str] =
    &["transfer", "transfer_from", "approve", "burn", "burn_from"];

pub fn detect(roots: &[AuthNode]) -> Vec<ProtocolHint> {
    let mut hints = Vec::new();
    let mut stack: Vec<&AuthNode> = roots.iter().collect();

    while let Some(node) = stack.pop() {
        if let Some(hint) = classify(node) {
            if !hints.iter().any(|h| hints_equal(h, &hint)) {
                hints.push(hint);
            }
        }
        for sub in &node.sub_invocations {
            stack.push(sub);
        }
    }

    hints
}

fn classify(node: &AuthNode) -> Option<ProtocolHint> {
    if SOROSWAP_FUNCTIONS.contains(&node.function.as_str()) {
        let path = extract_path(&node.args);
        return Some(ProtocolHint::Soroswap {
            router: node.contract.clone(),
            path,
        });
    }

    if BLEND_FUNCTIONS.contains(&node.function.as_str()) {
        let pool = extract_pool_from_args(&node.args)
            .unwrap_or_else(|| node.contract.clone());
        return Some(ProtocolHint::Blend {
            pool,
            backstop: node.contract.clone(),
        });
    }

    if SEP41_FUNCTIONS.contains(&node.function.as_str()) {
        return Some(ProtocolHint::Sep41 {
            contract: node.contract.clone(),
        });
    }

    Some(ProtocolHint::Unknown {
        contract: node.contract.clone(),
        function: node.function.clone(),
    })
}

fn extract_path(args: &[ScValIR]) -> Vec<String> {
    for arg in args {
        if let ScValIR::Vec(items) = arg {
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
            if addrs.len() >= 2 {
                return addrs;
            }
        }
    }
    Vec::new()
}

fn extract_pool_from_args(args: &[ScValIR]) -> Option<String> {
    for arg in args {
        if let ScValIR::Address(a) = arg {
            return Some(a.clone());
        }
    }
    None
}

fn hints_equal(a: &ProtocolHint, b: &ProtocolHint) -> bool {
    match (a, b) {
        (ProtocolHint::Soroswap { router: r1, .. }, ProtocolHint::Soroswap { router: r2, .. }) => {
            r1 == r2
        }
        (ProtocolHint::Blend { backstop: b1, .. }, ProtocolHint::Blend { backstop: b2, .. }) => {
            b1 == b2
        }
        (ProtocolHint::Sep41 { contract: c1 }, ProtocolHint::Sep41 { contract: c2 }) => c1 == c2,
        (
            ProtocolHint::Unknown { contract: c1, function: f1 },
            ProtocolHint::Unknown { contract: c2, function: f2 },
        ) => c1 == c2 && f1 == f2,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(contract: &str, function: &str, args: Vec<ScValIR>) -> AuthNode {
        AuthNode {
            signer: "signer".to_string(),
            contract: contract.to_string(),
            function: function.to_string(),
            args,
            sub_invocations: vec![],
        }
    }

    #[test]
    fn detects_soroswap() {
        let path_arg = ScValIR::Vec(vec![
            ScValIR::Address("USDC".to_string()),
            ScValIR::Address("XLM".to_string()),
        ]);
        let n = node("ROUTER", "swap_exact_tokens_for_tokens", vec![
            ScValIR::I128(1000),
            ScValIR::I128(900),
            path_arg,
            ScValIR::Address("DEST".to_string()),
            ScValIR::U64(999999),
        ]);
        let hints = detect(&[n]);
        assert_eq!(hints.len(), 1);
        assert!(matches!(hints[0], ProtocolHint::Soroswap { .. }));
    }

    #[test]
    fn detects_sep41() {
        let n = node("CTOKEN", "transfer", vec![
            ScValIR::Address("FROM".to_string()),
            ScValIR::Address("TO".to_string()),
            ScValIR::I128(500),
        ]);
        let hints = detect(&[n]);
        assert!(matches!(hints[0], ProtocolHint::Sep41 { .. }));
    }

    #[test]
    fn deduplicates_hints() {
        let n1 = node("CTOKEN", "transfer", vec![]);
        let n2 = node("CTOKEN", "transfer_from", vec![]);
        let hints = detect(&[n1, n2]);
        assert_eq!(hints.len(), 1);
    }
}
