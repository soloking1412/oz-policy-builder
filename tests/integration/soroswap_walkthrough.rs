use oz_policy_builder_core::{
    generate, run_harness, synthesize,
    ir::types::{
        ArgConstraint, AuthNode, HarnessVector, Outcome, PolicyKind, ProtocolHint, ScValIR,
        SynthesisOptions, TokenFlow, TransactionIR,
    },
};

const ROUTER: &str = "CSOROSWAPROU000000000000000000000000000000000000000000000";
const USDC: &str = "CUSDCTOKEN00000000000000000000000000000000000000000000000";
const XLM: &str = "CXLMTOKEN000000000000000000000000000000000000000000000000";
const SIGNER: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

fn soroswap_ir(amount_in: i128) -> TransactionIR {
    let swap_node = AuthNode {
        signer: SIGNER.to_string(),
        contract: ROUTER.to_string(),
        function: "swap_exact_tokens_for_tokens".to_string(),
        args: vec![
            ScValIR::I128(amount_in),
            ScValIR::I128((amount_in as f64 * 0.9) as i128),
            ScValIR::Vec(vec![
                ScValIR::Address(USDC.to_string()),
                ScValIR::Address(XLM.to_string()),
            ]),
            ScValIR::Address(SIGNER.to_string()),
            ScValIR::U64(9_999_999_999),
        ],
        sub_invocations: vec![],
    };

    TransactionIR {
        tx_hash: Some("soroswap_walkthrough".to_string()),
        source: SIGNER.to_string(),
        auth_roots: vec![swap_node],
        token_flows: vec![
            TokenFlow {
                token: USDC.to_string(),
                from: SIGNER.to_string(),
                to: "CSOROSWAPPAIR000000000000000000000000000000000000000000".to_string(),
                amount: amount_in,
            },
            TokenFlow {
                token: XLM.to_string(),
                from: "CSOROSWAPPAIR000000000000000000000000000000000000000000".to_string(),
                to: SIGNER.to_string(),
                amount: (amount_in as f64 * 0.93) as i128,
            },
        ],
        detected_protocols: vec![ProtocolHint::Soroswap {
            router: ROUTER.to_string(),
            path: vec![USDC.to_string(), XLM.to_string()],
        }],
    }
}

#[test]
fn synthesizes_custom_for_soroswap() {
    let ir = soroswap_ir(100_000_000);
    let opts = SynthesisOptions {
        context_rule_id: "soroswap_bounded".to_string(),
        ..Default::default()
    };
    let spec = synthesize(ir, opts).unwrap();
    assert_eq!(spec.kind, PolicyKind::Custom);
    assert!(spec.allowed_contracts.contains(&ROUTER.to_string()));

    let has_path = spec.allowed_calls.iter().any(|c| {
        c.arg_constraints
            .iter()
            .any(|a| matches!(a, ArgConstraint::PathContains { .. }))
    });
    assert!(has_path, "expected PathContains constraint for Soroswap");

    let has_deadline = spec.allowed_calls.iter().any(|c| {
        c.arg_constraints
            .iter()
            .any(|a| matches!(a, ArgConstraint::Deadline { .. }))
    });
    assert!(has_deadline, "expected Deadline constraint for Soroswap");
}

#[test]
fn generates_compilable_code_for_soroswap() {
    let ir = soroswap_ir(100_000_000);
    let opts = SynthesisOptions {
        context_rule_id: "soroswap_bounded".to_string(),
        ..Default::default()
    };
    let spec = synthesize(ir, opts).unwrap();
    let policy = generate(&spec, "soroswap-bounded-swap").unwrap();
    assert!(policy.lib_rs.contains("#![no_std]"));
    assert!(policy.lib_rs.contains("PathNotAllowed"));
}

#[test]
fn harness_blocks_path_deviation() {
    let ir = soroswap_ir(100_000_000);
    let opts = SynthesisOptions {
        context_rule_id: "soroswap_bounded".to_string(),
        ..Default::default()
    };
    let spec = synthesize(ir, opts).unwrap();

    let path_deviation = HarnessVector {
        id: "deny_path_with_unknown_token".to_string(),
        expected: Outcome::Deny,
        contract: ROUTER.to_string(),
        function: "swap_exact_tokens_for_tokens".to_string(),
        args: vec![
            ScValIR::I128(50_000_000),
            ScValIR::I128(45_000_000),
            ScValIR::Vec(vec![
                ScValIR::Address(USDC.to_string()),
                ScValIR::Address(XLM.to_string()),
                ScValIR::Address("CROGUE_TOKEN_000000000000000000000000000000000000000000".to_string()),
            ]),
            ScValIR::Address(SIGNER.to_string()),
            ScValIR::U64(9_999_999_999),
        ],
    };

    let report = run_harness(&spec, vec![], vec![path_deviation]).unwrap();
    assert_eq!(report.failed, 0, "path deviation should be denied");
}

#[test]
fn harness_blocks_excess_amount() {
    let ir = soroswap_ir(100_000_000);
    let opts = SynthesisOptions {
        context_rule_id: "soroswap_bounded".to_string(),
        ..Default::default()
    };
    let spec = synthesize(ir, opts).unwrap();

    let over_amount = HarnessVector {
        id: "deny_over_max_amount_in".to_string(),
        expected: Outcome::Deny,
        contract: ROUTER.to_string(),
        function: "swap_exact_tokens_for_tokens".to_string(),
        args: vec![ScValIR::I128(999_000_000_000)],
    };

    let report = run_harness(&spec, vec![], vec![over_amount]).unwrap();
    assert_eq!(report.failed, 0);
}
