use std::collections::HashMap;

use super::{Security, Tx};

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
