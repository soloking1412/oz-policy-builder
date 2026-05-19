use crate::ir::types::{AuthNode, ScValIR, TokenFlow};

pub fn extract_from_auth(roots: &[AuthNode]) -> Vec<TokenFlow> {
    let mut flows = Vec::new();
    let mut stack: Vec<&AuthNode> = roots.iter().collect();

    while let Some(node) = stack.pop() {
        if let Some(flow) = auth_node_to_flow(node) {
            flows.push(flow);
        }
        for sub in &node.sub_invocations {
            stack.push(sub);
        }
    }

    flows
}

fn auth_node_to_flow(node: &AuthNode) -> Option<TokenFlow> {
    match node.function.as_str() {
        "transfer" if node.args.len() >= 3 => {
            let from = addr_str(&node.args[0])?;
            let to = addr_str(&node.args[1])?;
            let amount = numeric_val(&node.args[2])?;
            Some(TokenFlow { token: node.contract.clone(), from, to, amount })
        }
        "transfer_from" if node.args.len() >= 4 => {
            let from = addr_str(&node.args[1])?;
            let to = addr_str(&node.args[2])?;
            let amount = numeric_val(&node.args[3])?;
            Some(TokenFlow { token: node.contract.clone(), from, to, amount })
        }
        _ => None,
    }
}

fn addr_str(v: &ScValIR) -> Option<String> {
    match v {
        ScValIR::Address(a) => Some(a.clone()),
        _ => None,
    }
}

fn numeric_val(v: &ScValIR) -> Option<i128> {
    match v {
        ScValIR::I128(n) => Some(*n),
        ScValIR::U128(n) => Some(*n as i128),
        ScValIR::U64(n) => Some(*n as i128),
        ScValIR::I64(n) => Some(*n as i128),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::types::ScValIR;

    fn make_transfer_node(token: &str, from: &str, to: &str, amount: i128) -> AuthNode {
        AuthNode {
            signer: from.to_string(),
            contract: token.to_string(),
            function: "transfer".to_string(),
            args: vec![
                ScValIR::Address(from.to_string()),
                ScValIR::Address(to.to_string()),
                ScValIR::I128(amount),
            ],
            sub_invocations: vec![],
        }
    }

    #[test]
    fn extracts_transfer_from_auth() {
        let node = make_transfer_node("CTOKEN", "ALICE", "BOB", 1000);
        let flows = extract_from_auth(&[node]);
        assert_eq!(flows.len(), 1);
        assert_eq!(flows[0].amount, 1000);
        assert_eq!(flows[0].from, "ALICE");
        assert_eq!(flows[0].to, "BOB");
    }

    #[test]
    fn extracts_transfer_from_function() {
        let node = AuthNode {
            signer: "SPENDER".to_string(),
            contract: "CTOKEN".to_string(),
            function: "transfer_from".to_string(),
            args: vec![
                ScValIR::Address("SPENDER".to_string()),
                ScValIR::Address("FROM".to_string()),
                ScValIR::Address("TO".to_string()),
                ScValIR::I128(500),
            ],
            sub_invocations: vec![],
        };
        let flows = extract_from_auth(&[node]);
        assert_eq!(flows.len(), 1);
        assert_eq!(flows[0].from, "FROM");
        assert_eq!(flows[0].to, "TO");
        assert_eq!(flows[0].amount, 500);
    }

    #[test]
    fn non_transfer_nodes_produce_no_flows() {
        let node = AuthNode {
            signer: "S".to_string(),
            contract: "C".to_string(),
            function: "claim".to_string(),
            args: vec![],
            sub_invocations: vec![],
        };
        let flows = extract_from_auth(&[node]);
        assert!(flows.is_empty());
    }
}
