use std::collections::HashSet;

use crate::error::Result;
use crate::ir::types::{
    AuthNode, PolicyKind, PolicyParameters, PolicySpec, SynthesisOptions, TransactionIR,
};
use crate::synthesizer::{allowlist, spending, threshold};

const PRIMITIVE_ELIGIBLE_FNS: &[&str] =
    &["transfer", "transfer_from", "approve", "burn", "burn_from"];

pub fn synthesize(ir: TransactionIR, opts: SynthesisOptions) -> Result<PolicySpec> {
    let mut log: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    let nodes = bfs_flatten(&ir.auth_roots);

    let signers: Vec<String> = {
        let mut seen = HashSet::new();
        nodes
            .iter()
            .filter(|n| seen.insert(n.signer.clone()))
            .map(|n| n.signer.clone())
            .collect()
    };

    log.push(format!(
        "observed {} auth node(s) across {} distinct signer(s)",
        nodes.len(),
        signers.len()
    ));

    if ir.token_flows.len() == 1 {
        warnings.push(
            "spending limit derived from a single observed transaction; \
             consider recording more samples before deploying"
                .to_string(),
        );
    }

    let only_token_ops = nodes
        .iter()
        .all(|n| PRIMITIVE_ELIGIBLE_FNS.contains(&n.function.as_str()));

    let dominant_token_params =
        spending::derive(&ir.token_flows, opts.spending_limit_multiplier, opts.window_ledgers);

    let kind = select_kind(
        &signers,
        &nodes,
        only_token_ops,
        dominant_token_params.is_some(),
        opts.force_custom,
        &mut log,
    );

    let parameters = match kind {
        PolicyKind::SpendingLimit => dominant_token_params.unwrap(),
        PolicyKind::SimpleThreshold => threshold::derive_simple(&signers),
        PolicyKind::WeightedThreshold => threshold::derive_weighted(&nodes),
        PolicyKind::Custom => PolicyParameters::Custom,
    };

    let allowed_calls = allowlist::build(&nodes, &ir.source, opts.spending_limit_multiplier);

    let allowed_contracts: Vec<String> = {
        let mut seen = HashSet::new();
        allowed_calls
            .iter()
            .filter(|c| seen.insert(c.contract.clone()))
            .map(|c| c.contract.clone())
            .collect()
    };

    log.push(format!(
        "selected policy kind: {:?}, {} allowed contract(s)",
        kind,
        allowed_contracts.len()
    ));

    if allowed_contracts.len() > 5 {
        warnings.push(
            "OZ smart accounts allow a maximum of 5 policies per context rule; \
             this spec references more contracts than recommended"
                .to_string(),
        );
    }

    Ok(PolicySpec {
        kind,
        context_rule_id: opts.context_rule_id,
        allowed_contracts,
        allowed_calls,
        parameters,
        reasoning: log,
        warnings,
    })
}

fn select_kind(
    signers: &[String],
    nodes: &[&AuthNode],
    only_token_ops: bool,
    has_spending_params: bool,
    force_custom: bool,
    log: &mut Vec<String>,
) -> PolicyKind {
    if force_custom {
        log.push("force_custom=true; generating custom policy".to_string());
        return PolicyKind::Custom;
    }

    if signers.len() == 1 && only_token_ops && has_spending_params {
        log.push(
            "single signer, token-only operations, spending limit derivable \
             → using spending_limit primitive"
                .to_string(),
        );
        return PolicyKind::SpendingLimit;
    }

    if signers.len() > 1 {
        let depths: HashSet<u32> = nodes
            .iter()
            .map(|n| invocation_depth(n))
            .collect();

        if depths.len() <= 1 {
            log.push(format!(
                "{} signers at equal depth → using simple_threshold primitive",
                signers.len()
            ));
            return PolicyKind::SimpleThreshold;
        } else {
            log.push(format!(
                "{} signers at varying depths → using weighted_threshold primitive",
                signers.len()
            ));
            return PolicyKind::WeightedThreshold;
        }
    }

    log.push(
        "protocol-specific functions detected → generating custom policy contract".to_string(),
    );
    PolicyKind::Custom
}

pub(super) fn invocation_depth(node: &AuthNode) -> u32 {
    if node.sub_invocations.is_empty() {
        0
    } else {
        1 + node.sub_invocations.iter().map(invocation_depth).max().unwrap_or(0)
    }
}

fn bfs_flatten(roots: &[AuthNode]) -> Vec<&AuthNode> {
    let mut result = Vec::new();
    let mut queue: std::collections::VecDeque<&AuthNode> = roots.iter().collect();
    while let Some(node) = queue.pop_front() {
        result.push(node);
        for sub in &node.sub_invocations {
            queue.push_back(sub);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::types::{ScValIR, TokenFlow};

    fn base_opts() -> SynthesisOptions {
        SynthesisOptions::default()
    }

    fn transfer_node(signer: &str, contract: &str, amount: i128) -> AuthNode {
        AuthNode {
            signer: signer.to_string(),
            contract: contract.to_string(),
            function: "transfer".to_string(),
            args: vec![
                ScValIR::Address(signer.to_string()),
                ScValIR::Address("DEST".to_string()),
                ScValIR::I128(amount),
            ],
            sub_invocations: vec![],
        }
    }

    fn ir_with_nodes(nodes: Vec<AuthNode>, flows: Vec<TokenFlow>) -> TransactionIR {
        TransactionIR {
            tx_hash: None,
            source: "SOURCE".to_string(),
            auth_roots: nodes,
            token_flows: flows,
            detected_protocols: vec![],
        }
    }

    fn flow(token: &str, amount: i128) -> TokenFlow {
        TokenFlow {
            token: token.to_string(),
            from: "SOURCE".to_string(),
            to: "DEST".to_string(),
            amount,
        }
    }

    #[test]
    fn single_signer_token_only_yields_spending_limit() {
        let ir = ir_with_nodes(
            vec![transfer_node("SOURCE", "USDC", 1000)],
            vec![flow("USDC", 1000)],
        );
        let spec = synthesize(ir, base_opts()).unwrap();
        assert_eq!(spec.kind, PolicyKind::SpendingLimit);
    }

    #[test]
    fn force_custom_overrides_primitive_selection() {
        let ir = ir_with_nodes(
            vec![transfer_node("SOURCE", "USDC", 1000)],
            vec![flow("USDC", 1000)],
        );
        let opts = SynthesisOptions { force_custom: true, ..base_opts() };
        let spec = synthesize(ir, opts).unwrap();
        assert_eq!(spec.kind, PolicyKind::Custom);
    }

    #[test]
    fn multi_signer_equal_depth_yields_simple_threshold() {
        let n1 = transfer_node("SIGNER_A", "USDC", 500);
        let n2 = transfer_node("SIGNER_B", "USDC", 500);
        let ir = ir_with_nodes(vec![n1, n2], vec![flow("USDC", 500)]);
        let spec = synthesize(ir, base_opts()).unwrap();
        assert_eq!(spec.kind, PolicyKind::SimpleThreshold);
    }

    #[test]
    fn protocol_specific_functions_yield_custom() {
        let n = AuthNode {
            signer: "SIGNER".to_string(),
            contract: "BACKSTOP".to_string(),
            function: "claim".to_string(),
            args: vec![],
            sub_invocations: vec![],
        };
        let ir = ir_with_nodes(vec![n], vec![]);
        let spec = synthesize(ir, base_opts()).unwrap();
        assert_eq!(spec.kind, PolicyKind::Custom);
    }

    #[test]
    fn single_observation_emits_warning() {
        let ir = ir_with_nodes(
            vec![transfer_node("SOURCE", "USDC", 1000)],
            vec![flow("USDC", 1000)],
        );
        let spec = synthesize(ir, base_opts()).unwrap();
        assert!(spec.warnings.iter().any(|w| w.contains("single observed transaction")));
    }

    #[test]
    fn allowed_contracts_are_deduplicated() {
        let n1 = transfer_node("SOURCE", "USDC", 100);
        let n2 = transfer_node("SOURCE", "USDC", 200);
        let ir = ir_with_nodes(vec![n1, n2], vec![flow("USDC", 200)]);
        let spec = synthesize(ir, base_opts()).unwrap();
        assert_eq!(spec.allowed_contracts.len(), 1);
    }

    #[test]
    fn multi_token_flows_escalate_to_custom() {
        let n = AuthNode {
            signer: "SOURCE".to_string(),
            contract: "ROUTER".to_string(),
            function: "swap_exact_tokens_for_tokens".to_string(),
            args: vec![
                ScValIR::I128(1000),
                ScValIR::I128(900),
                ScValIR::Vec(vec![
                    ScValIR::Address("USDC".to_string()),
                    ScValIR::Address("XLM".to_string()),
                ]),
                ScValIR::Address("DEST".to_string()),
                ScValIR::U64(999999),
            ],
            sub_invocations: vec![],
        };
        let ir = ir_with_nodes(vec![n], vec![flow("USDC", 1000), flow("XLM", 950)]);
        let spec = synthesize(ir, base_opts()).unwrap();
        assert_eq!(spec.kind, PolicyKind::Custom);
    }
}
