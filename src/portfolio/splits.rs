use crate::util::basic::SError;

use super::{find_all_non_global_affiliates, Affiliate, Tx, TxAction};

fn has_non_global_surrounding_splits(txs: &[Tx], idx: usize) -> bool {
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

            if tx.action() == TxAction::Split && !tx.affiliate.is_global() {
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

        if tx.action() == TxAction::Split && !tx.affiliate.is_global() {
            return true;
        }
    }

    false
}

/// Goes through sorted_security_txs and finds all Split Txs with the global
/// affiliate. Converts this into per-affiliate Txs, for all affiliates in
/// the vector.
/// As a sanity check, it will return an error if any candidate split is
/// accompanied by another within a day, which has a specified affiliate.
/// This would most likely occur if a user is following the old directions of
/// specifying a split for each affiliate, and would result in unexpected
/// behaviour because the "default" affiliate for splits is __global__ now,
/// and so it would result in a double split for those non-default affiliates,
/// which we do not want.
pub fn replace_global_security_splits(
    sorted_security_txs: &mut Vec<Tx>,
) -> Result<(), SError> {
    // First find all global splits and validate them
    let mut split_indices = Vec::new();

    // Collect indices of global splits and validate them
    for (idx, tx) in sorted_security_txs.iter().enumerate() {
        if tx.action() == TxAction::Split && tx.affiliate.is_global() {
            // Check for surrounding non-global splits
            if has_non_global_surrounding_splits(sorted_security_txs, idx) {
                return Err(format!(
                    "Found non-global split of {} near global split on {}. This likely indicates \
                    duplicate split entries. Manually specify Default for split if this is intentional.",
                    tx.security, tx.trade_date
                ));
            }
            split_indices.push(idx);
        }
    }

    // If no global splits found, nothing to do
    if split_indices.is_empty() {
        return Ok(());
    }

    // Get all affiliates we need to create splits for
    let mut non_global_affiliates: Vec<_> =
        find_all_non_global_affiliates(sorted_security_txs).into_iter().collect();

    // Ensure we have at least the default affiliate. This would be a weird case
    // where the only Txs are splits, but we'll handle it anyway.
    if non_global_affiliates.is_empty() {
        non_global_affiliates.push(Affiliate::default());
    }

    // Process splits in reverse order to not invalidate indices
    for &idx in split_indices.iter().rev() {
        if non_global_affiliates.len() == 1 {
            // If there's only one affiliate to replace with, just modify in place
            sorted_security_txs[idx].affiliate = non_global_affiliates[0].clone();
        } else {
            // For multiple affiliates, we need to remove and insert
            let global_split = sorted_security_txs.remove(idx);

            for affiliate in non_global_affiliates.iter().rev() {
                let mut new_split = global_split.clone();
                new_split.affiliate = affiliate.clone();
                sorted_security_txs.insert(idx, new_split);
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
        assert!(replace_global_security_splits(&mut txs).is_ok());
        assert!(txs.is_empty());
    }

    #[test]
    fn test_single_global_split_no_affiliates() {
        let mut txs = vec![create_tx(1, TxAction::Split, Affiliate::global(), 1)];

        replace_global_security_splits(&mut txs).unwrap();

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

        replace_global_security_splits(&mut txs).unwrap();

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

        replace_global_security_splits(&mut txs).unwrap();

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

        replace_global_security_splits(&mut txs).unwrap();

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

        replace_global_security_splits(&mut txs).unwrap();

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

        assert!(replace_global_security_splits(&mut txs).is_err());
        // Verify vector wasn't modified
        assert_eq!(txs.len(), 2);
        assert!(txs[0].affiliate.is_global());

        // On adjacent day
        let mut txs = vec![
            create_tx(1, TxAction::Split, Affiliate::global(), 1),
            create_tx(2, TxAction::Split, Affiliate::from_strep("test"), 2),
        ];

        assert!(replace_global_security_splits(&mut txs).is_err());
        // Verify vector wasn't modified
        assert_eq!(txs.len(), 2);
        assert!(txs[0].affiliate.is_global());

        let mut txs = vec![
            create_tx(1, TxAction::Split, Affiliate::from_strep("test"), 1),
            create_tx(2, TxAction::Split, Affiliate::global(), 2),
        ];

        assert!(replace_global_security_splits(&mut txs).is_err());
        // Verify vector wasn't modified
        assert_eq!(txs.len(), 2);
        assert!(txs[1].affiliate.is_global());

        // If we have no globals, no error
        let mut txs = vec![
            create_tx(1, TxAction::Split, Affiliate::from_strep("test"), 1),
            create_tx(1, TxAction::Split, Affiliate::from_strep("test2"), 2),
        ];

        let before_txs = txs.clone();
        assert!(replace_global_security_splits(&mut txs).is_ok());
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

        replace_global_security_splits(&mut txs).unwrap();

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

        replace_global_security_splits(&mut txs).unwrap();

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

        replace_global_security_splits(&mut txs).unwrap();

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
}
