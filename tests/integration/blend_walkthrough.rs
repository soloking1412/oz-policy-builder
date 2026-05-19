use oz_policy_builder_core::{
    generate, run_harness, synthesize,
    ir::types::{
        AuthNode, Outcome, PolicyKind, ProtocolHint, ScValIR, SynthesisOptions, TokenFlow,
    },
};

fn blend_ir() -> oz_policy_builder_core::ir::types::TransactionIR {
    let backstop = "CBLENDBACKSTOP000000000000000000000000000000000000000000";
    let pool = "CBLENDPOOL00000000000000000000000000000000000000000000000";
    let signer = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

    let claim_node = AuthNode {
        signer: signer.to_string(),
        contract: backstop.to_string(),
        function: "claim".to_string(),
        args: vec![
            ScValIR::Address(signer.to_string()),
            ScValIR::Vec(vec![ScValIR::Address(pool.to_string())]),
        ],
        sub_invocations: vec![],
    };

    let deposit_node = AuthNode {
        signer: signer.to_string(),
        contract: pool.to_string(),
        function: "deposit".to_string(),
        args: vec![
            ScValIR::Address(signer.to_string()),
            ScValIR::I128(1_500_000_000),
        ],
        sub_invocations: vec![],
    };

    oz_policy_builder_core::ir::types::TransactionIR {
        tx_hash: Some("blend_walkthrough".to_string()),
        source: signer.to_string(),
        auth_roots: vec![claim_node, deposit_node],
        token_flows: vec![TokenFlow {
            token: "CBLNDTOKEN00000000000000000000000000000000000000000000000".to_string(),
            from: backstop.to_string(),
            to: pool.to_string(),
            amount: 1_500_000_000,
        }],
        detected_protocols: vec![ProtocolHint::Blend {
            pool: pool.to_string(),
            backstop: backstop.to_string(),
        }],
    }
}

#[test]
fn synthesizes_custom_for_blend() {
    let ir = blend_ir();
    let opts = SynthesisOptions {
        context_rule_id: "blend_yield_daily".to_string(),
        ..Default::default()
    };
    let spec = synthesize(ir, opts).unwrap();
    assert_eq!(spec.kind, PolicyKind::Custom);
    assert!(spec.allowed_contracts.iter().any(|c| c.contains("BACKSTOP")));
    assert!(spec.allowed_contracts.iter().any(|c| c.contains("POOL")));
    assert!(spec
        .allowed_calls
        .iter()
        .any(|c| c.function == "claim"));
    assert!(spec
        .allowed_calls
        .iter()
        .any(|c| c.function == "deposit"));
}

#[test]
fn generates_compilable_code_for_blend() {
    let ir = blend_ir();
    let opts = SynthesisOptions {
        context_rule_id: "blend_yield_daily".to_string(),
        ..Default::default()
    };
    let spec = synthesize(ir, opts).unwrap();
    let policy = generate(&spec, "blend-yield-policy").unwrap();

    assert!(!policy.lib_rs.is_empty());
    assert!(!policy.cargo_toml.is_empty());
    assert!(policy.lib_rs.contains("#![no_std]"));
    assert!(policy.cargo_toml.contains("soroban-sdk"));
}

#[test]
fn harness_passes_for_blend_spec() {
    let ir = blend_ir();
    let opts = SynthesisOptions {
        context_rule_id: "blend_yield_daily".to_string(),
        ..Default::default()
    };
    let spec = synthesize(ir, opts).unwrap();
    let report = run_harness(&spec, vec![], vec![]).unwrap();
    assert_eq!(report.failed, 0, "failed harness vectors: {:?}",
        report.results.iter().filter(|r| !r.passed).collect::<Vec<_>>());
}

#[test]
fn deny_vectors_block_wrong_contract() {
    let ir = blend_ir();
    let opts = SynthesisOptions {
        context_rule_id: "blend_yield_daily".to_string(),
        ..Default::default()
    };
    let spec = synthesize(ir, opts).unwrap();

    use oz_policy_builder_core::ir::types::HarnessVector;
    let deny = HarnessVector {
        id: "deny_rogue_contract".to_string(),
        expected: Outcome::Deny,
        contract: "CROGUE00000000000000000000000000000000000000000000000000".to_string(),
        function: "claim".to_string(),
        args: vec![],
    };

    let report = run_harness(&spec, vec![], vec![deny]).unwrap();
    assert_eq!(report.failed, 0);
}
