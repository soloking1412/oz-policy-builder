use oz_policy_builder_core::{
    generate, run_harness, synthesize,
    ir::types::{
        ArgConstraint, AuthNode, HarnessVector, Outcome, ProtocolHint, ScValIR,
        SynthesisOptions, TokenFlow, TransactionIR,
    },
};

const TOKEN: &str = "CUSDCTOKEN00000000000000000000000000000000000000000000000";
const SUBSCRIBER: &str = "CSUBSCRIBER00000000000000000000000000000000000000000000";
const BENEFICIARY: &str = "CBENEFICIARY0000000000000000000000000000000000000000000";
const OWNER: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

fn sep41_ir() -> TransactionIR {
    let approve_node = AuthNode {
        signer: OWNER.to_string(),
        contract: TOKEN.to_string(),
        function: "approve".to_string(),
        args: vec![
            ScValIR::Address(OWNER.to_string()),
            ScValIR::Address(SUBSCRIBER.to_string()),
            ScValIR::I128(10_000_000),
            ScValIR::U64(200_000),
        ],
        sub_invocations: vec![],
    };

    let transfer_from_node = AuthNode {
        signer: SUBSCRIBER.to_string(),
        contract: TOKEN.to_string(),
        function: "transfer_from".to_string(),
        args: vec![
            ScValIR::Address(SUBSCRIBER.to_string()),
            ScValIR::Address(OWNER.to_string()),
            ScValIR::Address(BENEFICIARY.to_string()),
            ScValIR::I128(10_000_000),
        ],
        sub_invocations: vec![],
    };

    TransactionIR {
        tx_hash: Some("sep41_walkthrough".to_string()),
        source: OWNER.to_string(),
        auth_roots: vec![approve_node, transfer_from_node],
        token_flows: vec![TokenFlow {
            token: TOKEN.to_string(),
            from: OWNER.to_string(),
            to: BENEFICIARY.to_string(),
            amount: 10_000_000,
        }],
        detected_protocols: vec![ProtocolHint::Sep41 {
            contract: TOKEN.to_string(),
        }],
    }
}

#[test]
fn synthesizes_policy_for_subscription() {
    let ir = sep41_ir();
    let opts = SynthesisOptions {
        context_rule_id: "usdc_subscription".to_string(),
        ..Default::default()
    };
    let spec = synthesize(ir, opts).unwrap();

    assert!(spec.allowed_contracts.contains(&TOKEN.to_string()));
    assert!(spec.allowed_calls.iter().any(|c| c.function == "approve"));
    assert!(spec
        .allowed_calls
        .iter()
        .any(|c| c.function == "transfer_from"));
}

#[test]
fn generates_code_with_exact_address_constraints() {
    let ir = sep41_ir();
    let opts = SynthesisOptions {
        context_rule_id: "usdc_subscription".to_string(),
        ..Default::default()
    };
    let spec = synthesize(ir, opts).unwrap();

    let has_exact_addr = spec.allowed_calls.iter().any(|c| {
        c.arg_constraints
            .iter()
            .any(|a| matches!(a, ArgConstraint::ExactAddress { .. }))
    });
    assert!(has_exact_addr, "expected ExactAddress constraints for subscription");

    let policy = generate(&spec, "usdc-subscription").unwrap();
    assert!(policy.lib_rs.contains("#![no_std]"));
}

#[test]
fn harness_passes_all_vectors() {
    let ir = sep41_ir();
    let opts = SynthesisOptions {
        context_rule_id: "usdc_subscription".to_string(),
        ..Default::default()
    };
    let spec = synthesize(ir, opts).unwrap();
    let report = run_harness(&spec, vec![], vec![]).unwrap();
    assert_eq!(report.failed, 0, "failed: {:?}",
        report.results.iter().filter(|r| !r.passed).collect::<Vec<_>>());
}

#[test]
fn deny_vector_catches_wrong_beneficiary() {
    let ir = sep41_ir();
    let opts = SynthesisOptions {
        context_rule_id: "usdc_subscription".to_string(),
        ..Default::default()
    };
    let spec = synthesize(ir, opts).unwrap();

    let wrong_to = HarnessVector {
        id: "deny_wrong_beneficiary".to_string(),
        expected: Outcome::Deny,
        contract: TOKEN.to_string(),
        function: "transfer_from".to_string(),
        args: vec![
            ScValIR::Address(SUBSCRIBER.to_string()),
            ScValIR::Address(OWNER.to_string()),
            ScValIR::Address("CROGUE00000000000000000000000000000000000000000000000000".to_string()),
            ScValIR::I128(10_000_000),
        ],
    };

    let report = run_harness(&spec, vec![], vec![wrong_to]).unwrap();
    assert_eq!(report.failed, 0);
}
