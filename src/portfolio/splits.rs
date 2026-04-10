use crate::util::basic::SError;

use super::{
    find_all_non_global_affiliates, Affiliate, Tx, TxAction, TxActionSpecifics,
};

fn has_non_global_conflicting_surrounding_tx(
    txs: &[Tx],
    idx: usize,
    tx_action: TxAction,
) -> bool {
    let target_date = txs[idx].trade_date;

    // Check backwards
    let mut has_surrounding = false;
    if idx > 0 {
        for i in (0..idx).rev() {
            let tx = &txs[i];
            let days_diff = (target_date - tx.trade_date).whole_days();

            // If we're beyond our 1-day window, we can stop looking backwards
            if days_diff > 1 {
                break;
            }

            if tx.action() == tx_action && !tx.affiliate.is_global() {
                has_surrounding = true;
                break;
            }
        }
    }

    // If we already found a surrounding split, no need to check forward
    if has_surrounding {
        return true;
    }

    // Check forwards
    for tx in &txs[idx + 1..] {
        let days_diff = (tx.trade_date - target_date).whole_days();

        // If we're beyond our 1-day window, we can stop looking forwards
        if days_diff > 1 {
            break;
        }

        if tx.action() == tx_action && !tx.affiliate.is_global() {
            return true;
        }
    }

    false
}

fn is_per_share_dist(tx: &Tx) -> bool {
    match &tx.action_specifics {
        TxActionSpecifics::Roc(d)
        | TxActionSpecifics::RiCGDist(d)
        | TxActionSpecifics::RiDiv(d)
        | TxActionSpecifics::CGDiv(d) => d.amount_per_held_share().is_some(),
        _ => false,
    }
}

fn is_global_per_share_dist(tx: &Tx) -> bool {
    tx.affiliate.is_global() && is_per_share_dist(tx)
}

/// Goes through sorted_security_txs and finds all Split and per-share dist Txs
/// with the global affiliate, and converts each into per-affiliate Txs.
///
/// For splits, expands to all non-global affiliates (including registered),
/// since a split affects all share holders regardless of account type.
///
/// For per-share dists, expands to all non-global, non-registered affiliates
/// only, since dists cannot be applied to registered affiliates.
///
/// As a sanity check, returns an error if any candidate tx is accompanied by
/// a non-global tx of the same action within a day. This would most likely
/// indicate duplicate entries, and would result in double-counting.
pub fn replace_global_security_txs(
    sorted_security_txs: &mut Vec<Tx>,
) -> Result<(), SError> {
    let mut global_tx_indices = Vec::new();

    for (idx, tx) in sorted_security_txs.iter().enumerate() {
        let is_global_split =
            tx.action() == TxAction::Split && tx.affiliate.is_global();
        let is_global_dist = is_global_per_share_dist(tx);

        if is_global_split || is_global_dist {
            let action = tx.action();
            let has_conflict = has_non_global_conflicting_surrounding_tx(
                sorted_security_txs,
                idx,
                action,
            );
            if has_conflict {
                return Err(format!(
                    "Found non-global {} of {} near global {} on {}. This likely \
                    indicates duplicate entries. Manually specify Default if this \
                    is intentional.",
                    action.med_pretty_str(),
                    tx.security,
                    action.med_pretty_str(),
                    tx.trade_date
                ));
            }
            global_tx_indices.push(idx);
        }
    }

    if global_tx_indices.is_empty() {
        return Ok(());
    }

    // Precompute the two affiliate sets used during expansion.
    let all_non_global: Vec<_> =
        find_all_non_global_affiliates(sorted_security_txs).into_iter().collect();
    let non_registered: Vec<_> =
        all_non_global.iter().filter(|af| !af.registered()).cloned().collect();
    let just_default_affiliate = vec![Affiliate::default()];

    // Process in reverse order to not invalidate earlier indices.
    for &idx in global_tx_indices.iter().rev() {
        let is_split = sorted_security_txs[idx].action() == TxAction::Split;

        // Splits replicate for all affiliates (registered accounts still split).
        // Dists replicate only for non-registered affiliates.
        let base_set = if is_split {
            &all_non_global
        } else {
            &non_registered
        };
        let affiliates: &Vec<Affiliate> = if base_set.is_empty() {
            &just_default_affiliate
        } else {
            base_set
        };

        if affiliates.len() == 1 {
            // Modify in place rather than remove + insert.
            sorted_security_txs[idx].affiliate = affiliates[0].clone();
        } else {
            let global_tx = sorted_security_txs.remove(idx);
            for affiliate in affiliates.iter().rev() {
                let mut new_tx = global_tx.clone();
                new_tx.affiliate = affiliate.clone();
                sorted_security_txs.insert(idx, new_tx);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        portfolio::{testlib::TTx, SplitRatio},
        testlib::assert_vec_eq,
    };

    use super::*;
    use std::collections::HashMap;

    use crate::gezdec as gez;

    // Helper function to create a test transaction
    fn create_tx(
        date: i32, // Days since epoch for simplicity
        action_type: TxAction,
        affiliate: Affiliate,
        read_index: u32,
    ) -> Tx {
        let mut ttx = TTx {
            t_day: date,
            act: action_type,
            shares: gez!(1),
            price: gez!(1),
            af: affiliate,
            read_index,
            ..TTx::default()
        };
        if action_type == TxAction::Split {
            ttx.split = Some(SplitRatio::parse("2-for-1").unwrap());
        }
        ttx.x()
    }

    fn assert_tx(
        actual: &Tx,
        action: TxAction,
        affiliate: &Affiliate,
        read_index: u32,
    ) {
        assert_eq!(actual.action(), action);
        assert_eq!(&actual.affiliate, affiliate);
        assert_eq!(actual.read_index, read_index);
    }

    fn assert_affiliates_at_indices(
        txs: &Vec<Tx>,
        indices: Vec<usize>,
        affiliates: Vec<Affiliate>,
    ) {
        assert_eq!(
            indices.len(),
            affiliates.len(),
            "Number of indices must match number of affiliates"
        );

        // Verify indices are in bounds
        for idx in &indices {
            assert!(*idx < txs.len(), "Index {} is out of bounds", idx);
        }

        // Get the actual affiliates at the given indices
        let actual_affiliates: Vec<_> =
            indices.iter().map(|&idx| txs[idx].affiliate.clone()).collect();

        // Count occurrences of each affiliate in both vectors
        let mut expected_counts: HashMap<&Affiliate, usize> = HashMap::new();
        for affiliate in &affiliates {
            *expected_counts.entry(affiliate).or_default() += 1;
        }

        let mut actual_counts: HashMap<&Affiliate, usize> = HashMap::new();
        for affiliate in &actual_affiliates {
            *actual_counts.entry(affiliate).or_default() += 1;
        }

        assert_eq!(actual_counts, expected_counts,
            "Affiliate counts at indices {:?} don't match expected.\nFound counts: {:?}\nExpected counts: {:?}",
            indices, actual_counts, expected_counts);
    }

    #[test]
    fn test_empty_vector() {
        let mut txs = Vec::new();
        assert!(replace_global_security_txs(&mut txs).is_ok());
        assert!(txs.is_empty());
    }

    #[test]
    fn test_single_global_split_no_affiliates() {
        let mut txs = vec![create_tx(1, TxAction::Split, Affiliate::global(), 1)];

        replace_global_security_txs(&mut txs).unwrap();

        assert_eq!(txs.len(), 1);
        assert!(!txs[0].affiliate.is_global());
        assert_eq!(txs[0].affiliate, Affiliate::default());
        assert_eq!(txs[0].read_index, 1);
    }

    #[test]
    fn test_single_global_split_one_affiliate() {
        let affiliate = Affiliate::from_strep("test");
        let mut txs = vec![
            create_tx(1, TxAction::Buy, affiliate.clone(), 1),
            create_tx(1, TxAction::Split, Affiliate::global(), 2),
        ];

        replace_global_security_txs(&mut txs).unwrap();

        assert_eq!(txs.len(), 2);
        assert_eq!(txs[1].affiliate, affiliate);
        assert_eq!(txs[1].read_index, 2);
    }

    #[test]
    fn test_single_global_split_multiple_affiliates() {
        let affiliate1 = Affiliate::from_strep("test1");
        let affiliate2 = Affiliate::from_strep("test2");
        let mut txs = vec![
            create_tx(1, TxAction::Buy, affiliate1.clone(), 1),
            create_tx(1, TxAction::Split, Affiliate::global(), 2),
            create_tx(1, TxAction::Buy, affiliate2.clone(), 3),
        ];

        replace_global_security_txs(&mut txs).unwrap();

        assert_eq!(txs.len(), 4);
        assert_affiliates_at_indices(
            &txs,
            vec![1, 2],
            vec![affiliate1.clone(), affiliate2.clone()],
        );
        assert_eq!(txs[1].read_index, 2);
        assert_eq!(txs[2].read_index, 2);
    }

    #[test]
    fn test_multiple_global_splits() {
        let affiliate1 = Affiliate::from_strep("test1");
        let affiliate2 = Affiliate::from_strep("test2");

        let mut txs = vec![
            create_tx(1, TxAction::Split, Affiliate::global(), 1),
            create_tx(10, TxAction::Split, Affiliate::global(), 2),
            create_tx(11, TxAction::Buy, affiliate1.clone(), 3),
        ];

        replace_global_security_txs(&mut txs).unwrap();

        assert_eq!(txs.len(), 3);
        assert_tx(&txs[0], TxAction::Split, &affiliate1, 1);
        assert_tx(&txs[1], TxAction::Split, &affiliate1, 2);
        assert_tx(&txs[2], TxAction::Buy, &affiliate1, 3);

        // Multiple affiliates
        let mut txs = vec![
            create_tx(1, TxAction::Split, Affiliate::global(), 1),
            create_tx(10, TxAction::Split, Affiliate::global(), 2),
            create_tx(11, TxAction::Buy, affiliate1.clone(), 3),
            create_tx(12, TxAction::Buy, affiliate2.clone(), 4),
        ];

        replace_global_security_txs(&mut txs).unwrap();

        assert_eq!(txs.len(), 6);
        assert_affiliates_at_indices(
            &txs,
            vec![0, 1],
            vec![affiliate1.clone(), affiliate2.clone()],
        );
        assert_eq!(txs[0].read_index, 1);
        assert_eq!(txs[1].read_index, 1);
        assert_affiliates_at_indices(
            &txs,
            vec![2, 3],
            vec![affiliate1.clone(), affiliate2.clone()],
        );
        assert_eq!(txs[2].read_index, 2);
        assert_eq!(txs[3].read_index, 2);
    }

    #[test]
    fn test_nearby_non_global_split_error() {
        // On the same day
        let mut txs = vec![
            create_tx(1, TxAction::Split, Affiliate::global(), 1),
            create_tx(1, TxAction::Split, Affiliate::from_strep("test"), 2),
        ];

        assert!(replace_global_security_txs(&mut txs).is_err());
        // Verify vector wasn't modified
        assert_eq!(txs.len(), 2);
        assert!(txs[0].affiliate.is_global());

        // On adjacent day
        let mut txs = vec![
            create_tx(1, TxAction::Split, Affiliate::global(), 1),
            create_tx(2, TxAction::Split, Affiliate::from_strep("test"), 2),
        ];

        assert!(replace_global_security_txs(&mut txs).is_err());
        // Verify vector wasn't modified
        assert_eq!(txs.len(), 2);
        assert!(txs[0].affiliate.is_global());

        let mut txs = vec![
            create_tx(1, TxAction::Split, Affiliate::from_strep("test"), 1),
            create_tx(2, TxAction::Split, Affiliate::global(), 2),
        ];

        assert!(replace_global_security_txs(&mut txs).is_err());
        // Verify vector wasn't modified
        assert_eq!(txs.len(), 2);
        assert!(txs[1].affiliate.is_global());

        // If we have no globals, no error
        let mut txs = vec![
            create_tx(1, TxAction::Split, Affiliate::from_strep("test"), 1),
            create_tx(1, TxAction::Split, Affiliate::from_strep("test2"), 2),
        ];

        let before_txs = txs.clone();
        assert!(replace_global_security_txs(&mut txs).is_ok());
        // Verify vector wasn't modified
        assert_vec_eq(before_txs, txs);
    }

    #[test]
    fn test_splits_outside_window() {
        let mut txs = vec![
            create_tx(1, TxAction::Split, Affiliate::from_strep("test1"), 1),
            create_tx(3, TxAction::Split, Affiliate::global(), 2),
            create_tx(5, TxAction::Split, Affiliate::from_strep("test2"), 3),
        ];

        replace_global_security_txs(&mut txs).unwrap();

        assert_eq!(txs.len(), 4);
        assert!(!txs[1].affiliate.is_global());
    }

    #[test]
    fn test_edge_cases() {
        let affiliate1 = Affiliate::from_strep("test1");
        let affiliate2 = Affiliate::from_strep("test2");

        // Test global splits at start, end, and only global splits
        let mut txs = vec![
            create_tx(1, TxAction::Split, Affiliate::global(), 1),
            create_tx(10, TxAction::Buy, affiliate1.clone(), 2),
            create_tx(20, TxAction::Split, Affiliate::global(), 3),
        ];

        replace_global_security_txs(&mut txs).unwrap();

        assert_eq!(txs.len(), 3);
        assert!(!txs[0].affiliate.is_global());
        assert!(!txs[2].affiliate.is_global());
        assert_eq!(txs[1].action(), TxAction::Buy); // Non-split transaction unchanged

        // Two affiliates
        let mut txs = vec![
            create_tx(1, TxAction::Split, Affiliate::global(), 1),
            create_tx(10, TxAction::Buy, affiliate1.clone(), 2),
            create_tx(10, TxAction::Buy, affiliate2.clone(), 3),
            create_tx(20, TxAction::Split, Affiliate::global(), 4),
        ];

        replace_global_security_txs(&mut txs).unwrap();

        assert_eq!(txs.len(), 6);
        assert_affiliates_at_indices(
            &txs,
            vec![0, 1],
            vec![affiliate1.clone(), affiliate2.clone()],
        );
        assert_affiliates_at_indices(
            &txs,
            vec![4, 5],
            vec![affiliate1.clone(), affiliate2.clone()],
        );
        assert_eq!(txs[0].action(), TxAction::Split);
        assert_eq!(txs[1].action(), TxAction::Split);
        assert_eq!(txs[2].action(), TxAction::Buy);
        assert_eq!(txs[3].action(), TxAction::Buy);
        assert_eq!(txs[4].action(), TxAction::Split);
        assert_eq!(txs[5].action(), TxAction::Split);
    }

    // Helper to create a per-share dist tx (global affiliate by default)
    fn create_dist_tx(
        date: i32,
        action_type: TxAction,
        affiliate: Affiliate,
        read_index: u32,
    ) -> Tx {
        TTx {
            t_day: date,
            act: action_type,
            price: gez!(1), // amount_per_share → triggers global default
            af: affiliate,
            read_index,
            ..TTx::default()
        }
        .x()
    }

    #[test]
    fn test_global_dist_no_affiliates() {
        // A global per-share dist with no other affiliates → defaults to Default
        for action in
            [TxAction::Roc, TxAction::RiCGDist, TxAction::RiDiv, TxAction::CGDiv]
        {
            let mut txs = vec![create_dist_tx(1, action, Affiliate::global(), 1)];

            replace_global_security_txs(&mut txs).unwrap();

            assert_eq!(txs.len(), 1);
            assert_eq!(txs[0].affiliate, Affiliate::default());
            assert_eq!(txs[0].read_index, 1);
        }
    }

    #[test]
    fn test_global_dist_one_non_registered_affiliate() {
        let affiliate = Affiliate::from_strep("myaf");
        for action in
            [TxAction::Roc, TxAction::RiCGDist, TxAction::RiDiv, TxAction::CGDiv]
        {
            let mut txs = vec![
                create_tx(1, TxAction::Buy, affiliate.clone(), 1),
                create_dist_tx(2, action, Affiliate::global(), 2),
            ];

            replace_global_security_txs(&mut txs).unwrap();

            assert_eq!(txs.len(), 2);
            assert_eq!(txs[1].affiliate, affiliate);
            assert_eq!(txs[1].read_index, 2);
        }
    }

    #[test]
    fn test_global_dist_multiple_non_registered_affiliates() {
        let af1 = Affiliate::from_strep("af1");
        let af2 = Affiliate::from_strep("af2");
        for action in
            [TxAction::Roc, TxAction::RiCGDist, TxAction::RiDiv, TxAction::CGDiv]
        {
            let mut txs = vec![
                create_tx(1, TxAction::Buy, af1.clone(), 1),
                create_tx(1, TxAction::Buy, af2.clone(), 2),
                create_dist_tx(2, action, Affiliate::global(), 3),
            ];

            replace_global_security_txs(&mut txs).unwrap();

            // One dist tx per non-registered affiliate
            assert_eq!(txs.len(), 4);
            assert_affiliates_at_indices(
                &txs,
                vec![2, 3],
                vec![af1.clone(), af2.clone()],
            );
            assert_eq!(txs[2].read_index, 3);
            assert_eq!(txs[3].read_index, 3);
        }
    }

    #[test]
    fn test_global_dist_skips_registered_affiliates() {
        // Registered affiliate should not receive a dist tx
        let af_nonreg = Affiliate::from_strep("nonreg");
        let af_reg = Affiliate::default_registered();
        for action in
            [TxAction::Roc, TxAction::RiCGDist, TxAction::RiDiv, TxAction::CGDiv]
        {
            let mut txs = vec![
                create_tx(1, TxAction::Buy, af_nonreg.clone(), 1),
                create_tx(1, TxAction::Buy, af_reg.clone(), 2),
                create_dist_tx(2, action, Affiliate::global(), 3),
            ];

            replace_global_security_txs(&mut txs).unwrap();

            // Only one dist tx — for the non-registered affiliate
            assert_eq!(txs.len(), 3);
            assert_eq!(txs[2].affiliate, af_nonreg);
        }
    }

    #[test]
    fn test_global_dist_all_four_affiliate_types() {
        // Buys from default, default registered, non-default, and non-default
        // registered. Only the two non-registered affiliates should receive
        // copies of the dist.
        let af_default = Affiliate::default();
        let af_default_reg = Affiliate::default_registered();
        let af_nondefault = Affiliate::from_strep("spouse");
        let af_nondefault_reg = Affiliate::from_base_name("spouse", true);

        for action in
            [TxAction::Roc, TxAction::RiCGDist, TxAction::RiDiv, TxAction::CGDiv]
        {
            let mut txs = vec![
                create_tx(1, TxAction::Buy, af_default.clone(), 1),
                create_tx(1, TxAction::Buy, af_default_reg.clone(), 2),
                create_tx(1, TxAction::Buy, af_nondefault.clone(), 3),
                create_tx(1, TxAction::Buy, af_nondefault_reg.clone(), 4),
                create_dist_tx(2, action, Affiliate::global(), 5),
            ];

            replace_global_security_txs(&mut txs).unwrap();

            // One dist tx per non-registered affiliate (default + nondefault = 2)
            assert_eq!(txs.len(), 6);
            assert_affiliates_at_indices(
                &txs,
                vec![4, 5],
                vec![af_default.clone(), af_nondefault.clone()],
            );
            assert_eq!(txs[4].read_index, 5);
            assert_eq!(txs[5].read_index, 5);
        }
    }

    #[test]
    fn test_global_dist_total_amount_not_expanded() {
        // A dist with total_amount (not per-share) should NOT be expanded
        for action in
            [TxAction::Roc, TxAction::RiCGDist, TxAction::RiDiv, TxAction::CGDiv]
        {
            let total_dist_tx = TTx {
                t_day: 2,
                act: action,
                t_amt: gez!(5), // total amount, not per-share
                af: Affiliate::global(),
                read_index: 2,
                ..TTx::default()
            }
            .x();
            let mut txs = vec![
                create_tx(1, TxAction::Buy, Affiliate::from_strep("af1"), 1),
                total_dist_tx,
            ];
            let before = txs.clone();

            replace_global_security_txs(&mut txs).unwrap();

            // Global total-amount dists are left untouched
            assert_vec_eq(before, txs);
        }
    }

    #[test]
    fn test_nearby_non_global_dist_error() {
        for action in
            [TxAction::Roc, TxAction::RiCGDist, TxAction::RiDiv, TxAction::CGDiv]
        {
            // Same day: global per-share + explicit per-share -> error (duplicate)
            let mut txs = vec![
                create_dist_tx(1, action, Affiliate::global(), 1),
                create_dist_tx(1, action, Affiliate::from_strep("test"), 2),
            ];
            assert!(replace_global_security_txs(&mut txs).is_err());
            assert!(txs[0].affiliate.is_global());

            // Adjacent day: global per-share + explicit per-share -> error
            let mut txs = vec![
                create_dist_tx(1, action, Affiliate::global(), 1),
                create_dist_tx(2, action, Affiliate::from_strep("test"), 2),
            ];
            assert!(replace_global_security_txs(&mut txs).is_err());
            assert!(txs[0].affiliate.is_global());

            // Same day: global per-share + total-amount (unrealistic) -> error
            // (total-amount is a different kind of transaction, not a duplicate)
            let total_dist_tx = TTx {
                t_day: 1,
                act: action,
                t_amt: gez!(5),
                af: Affiliate::from_strep("test"),
                read_index: 2,
                ..TTx::default()
            }
            .x();
            let mut txs = vec![
                create_dist_tx(1, action, Affiliate::global(), 1),
                total_dist_tx,
            ];
            assert!(replace_global_security_txs(&mut txs).is_err());
            assert!(txs[0].affiliate.is_global());

            // Different dist action types on same day — not an error
            let other_action = match action {
                TxAction::Roc => TxAction::RiDiv,
                _ => TxAction::Roc,
            };
            let mut txs = vec![
                create_dist_tx(1, action, Affiliate::global(), 1),
                create_dist_tx(1, other_action, Affiliate::from_strep("test"), 2),
            ];
            assert!(replace_global_security_txs(&mut txs).is_ok());
        }
    }
}
