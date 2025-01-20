use std::collections::{HashMap, HashSet};

use super::{Affiliate, Security, Tx};

pub fn split_txs_by_security(all_txs: Vec<Tx>) -> HashMap<Security, Vec<Tx>> {
    let mut txs_by_sec = HashMap::new();
    for tx in all_txs {
        if !txs_by_sec.contains_key(&tx.security) {
            txs_by_sec.insert(tx.security.clone(), Vec::new());
        }
        txs_by_sec.get_mut(&tx.security).unwrap().push(tx);
    }

    txs_by_sec
}

pub fn find_all_non_global_affiliates(txs: &Vec<Tx>) -> HashSet<Affiliate> {
    let mut aff_set = HashSet::new();
    for tx in txs {
        if !tx.affiliate.is_global() && !aff_set.contains(&tx.affiliate) {
            aff_set.insert(tx.affiliate.clone());
        }
    }
    aff_set
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::{gezdec, portfolio::{find_all_non_global_affiliates, testlib::TTx, Affiliate, TxAction}};

    #[test]
    #[rustfmt::skip]
    fn test_find_all_affiliates() {
        let txs = vec![
            TTx{t_day: 10, act: TxAction::Sell, shares: gezdec!(1), price: gezdec!(1),
                af: Affiliate::default(), ..TTx::d()}.x(),
            TTx{t_day: 10, act: TxAction::Sell, shares: gezdec!(1), price: gezdec!(1),
                af: Affiliate::default(), ..TTx::d()}.x(),
            TTx{t_day: 10, act: TxAction::Sell, shares: gezdec!(1), price: gezdec!(1),
                af: Affiliate::default_registered(), ..TTx::d()}.x(),
            // This is technically illegal. It should never happen for a Sell
            TTx{t_day: 10, act: TxAction::Sell, shares: gezdec!(1), price: gezdec!(1),
                af: Affiliate::global(), ..TTx::d()}.x(),
        ];

        let affs = find_all_non_global_affiliates(&txs);
        assert_eq!(
            affs,
            HashSet::<Affiliate>::from_iter(vec![
                Affiliate::default(), Affiliate::default_registered()])
        )
    }
}