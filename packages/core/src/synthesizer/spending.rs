use crate::ir::types::{PolicyParameters, TokenFlow};

pub fn derive(flows: &[TokenFlow], multiplier: f64, window_ledgers: u32) -> Option<PolicyParameters> {
    if flows.is_empty() {
        return None;
    }

    let tokens: std::collections::HashSet<&str> =
        flows.iter().map(|f| f.token.as_str()).collect();

    if tokens.len() != 1 {
        return None;
    }

    let token = flows[0].token.clone();
    let max_amount = flows.iter().map(|f| f.amount.abs()).max().unwrap_or(0);
    let limit = (max_amount as f64 * multiplier).ceil() as i128;

    Some(PolicyParameters::SpendingLimit {
        token,
        limit,
        window_ledgers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flow(token: &str, amount: i128) -> TokenFlow {
        TokenFlow {
            token: token.to_string(),
            from: "A".to_string(),
            to: "B".to_string(),
            amount,
        }
    }

    #[test]
    fn single_token_produces_spending_limit() {
        let flows = vec![flow("USDC", 1000)];
        let p = derive(&flows, 1.1, 17280).unwrap();
        assert!(matches!(p, PolicyParameters::SpendingLimit { limit: 1100, .. }));
    }

    #[test]
    fn multi_token_returns_none() {
        let flows = vec![flow("USDC", 1000), flow("XLM", 500)];
        assert!(derive(&flows, 1.1, 17280).is_none());
    }

    #[test]
    fn empty_flows_returns_none() {
        assert!(derive(&[], 1.1, 17280).is_none());
    }

    #[test]
    fn uses_largest_observed_amount() {
        let flows = vec![flow("USDC", 500), flow("USDC", 2000), flow("USDC", 1000)];
        let p = derive(&flows, 1.0, 17280).unwrap();
        assert!(matches!(p, PolicyParameters::SpendingLimit { limit: 2000, .. }));
    }
}
