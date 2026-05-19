use crate::ir::types::{AuthNode, PolicyParameters};
use super::decision::invocation_depth;

pub fn derive_simple(signers: &[String]) -> PolicyParameters {
    PolicyParameters::SimpleThreshold {
        threshold: signers.len() as u32,
    }
}

pub fn derive_weighted(nodes: &[&AuthNode]) -> PolicyParameters {
    let mut signer_depth: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    for node in nodes {
        let depth = invocation_depth(node);
        signer_depth
            .entry(node.signer.clone())
            .and_modify(|d| *d = (*d).max(depth))
            .or_insert(depth);
    }

    let max_depth = signer_depth.values().copied().max().unwrap_or(1).max(1);
    let signers: Vec<(String, u32)> = signer_depth
        .into_iter()
        .map(|(s, d)| (s, max_depth - d + 1))
        .collect();

    let threshold: u32 = signers.iter().map(|(_, w)| w).sum();

    PolicyParameters::WeightedThreshold { signers, threshold }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(signer: &str, subs: Vec<AuthNode>) -> AuthNode {
        AuthNode {
            signer: signer.to_string(),
            contract: "C".to_string(),
            function: "f".to_string(),
            args: vec![],
            sub_invocations: subs,
        }
    }

    #[test]
    fn simple_threshold_matches_signer_count() {
        let signers = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let p = derive_simple(&signers);
        assert!(matches!(p, PolicyParameters::SimpleThreshold { threshold: 3 }));
    }

    #[test]
    fn weighted_assigns_higher_weight_to_shallower_signers() {
        let deep = node("deep_signer", vec![node("inner", vec![])]);
        let shallow = node("shallow_signer", vec![]);
        let nodes: Vec<&AuthNode> = vec![&deep, &shallow];
        let p = derive_weighted(&nodes);
        if let PolicyParameters::WeightedThreshold { signers, .. } = p {
            let shallow_w = signers.iter().find(|(s, _)| s == "shallow_signer").map(|(_, w)| *w).unwrap();
            let deep_w = signers.iter().find(|(s, _)| s == "deep_signer").map(|(_, w)| *w).unwrap();
            assert!(shallow_w >= deep_w);
        } else {
            panic!("expected WeightedThreshold");
        }
    }
}
