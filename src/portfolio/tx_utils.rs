use std::collections::{HashMap, HashSet};

use crate::portfolio::model::tx::CsvTx;

/// Result of resolving a symbol through a rename map recursively.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameChain {
    /// The final symbol after applying all rename hops.
    pub resolved: String,
    /// Human-readable chain, e.g. `"G036247 AKA DLR.U.TO AKA DLR.TO"`.
    /// Always includes the origin and the final resolved symbol.
    pub chain_repr: String,
}

/// Recursively resolves `sym` through `renames`. Returns `None` if `sym` is
/// not a key in the map (no rename to apply). Otherwise follows the chain to
/// the final symbol and produces an `AKA`-joined chain string.
///
/// Cycles are broken: if a hop revisits an already-seen symbol, resolution
/// stops at the previous hop.
pub fn resolve_rename(
    sym: &str,
    renames: &HashMap<String, String>,
) -> Option<RenameChain> {
    if !renames.contains_key(sym) {
        return None;
    }
    let mut chain: Vec<String> = vec![sym.to_string()];
    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(sym.to_string());
    let mut current = sym.to_string();
    while let Some(next) = renames.get(&current) {
        if !visited.insert(next.clone()) {
            break;
        }
        chain.push(next.clone());
        current = next.clone();
    }
    Some(RenameChain {
        resolved: current,
        chain_repr: chain.join(" AKA "),
    })
}

/// Apply a symbol rename to a single `CsvTx`, updating `security` and `memo`.
///
/// Looks up `tx.security` in `renames`; if found, follows the chain to its
/// final symbol and appends an `AKA`-joined chain string to the memo (e.g.
/// `"G036247 AKA DLR.U.TO AKA DLR.TO"`). No-op if `tx.security` is `None` or
/// not present in the map.
pub fn apply_security_rename(tx: &mut CsvTx, renames: &HashMap<String, String>) {
    if let Some(sec) = tx.security.take() {
        if let Some(rc) = resolve_rename(&sec, renames) {
            match &mut tx.memo {
                Some(m) if !m.is_empty() => {
                    m.push_str(" ; ");
                    m.push_str(&rc.chain_repr);
                }
                Some(m) => *m = rc.chain_repr,
                None => tx.memo = Some(rc.chain_repr),
            }
            tx.security = Some(rc.resolved);
        } else {
            tx.security = Some(sec);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portfolio::model::tx::CsvTx;

    fn renames(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(f, t)| (f.to_string(), t.to_string())).collect()
    }

    #[test]
    fn test_resolve_rename_no_match() {
        let r = renames(&[("FOO", "FOO.TO")]);
        assert!(resolve_rename("BAR", &r).is_none());
    }

    #[test]
    fn test_resolve_rename_single_hop() {
        let r = renames(&[("FOO", "FOO.TO")]);
        let rc = resolve_rename("FOO", &r).unwrap();
        assert_eq!(rc.resolved, "FOO.TO");
        assert_eq!(rc.chain_repr, "FOO AKA FOO.TO");
    }

    #[test]
    fn test_resolve_rename_chain() {
        let r = renames(&[("G036247", "DLR.U.TO"), ("DLR.U.TO", "DLR.TO")]);
        let rc = resolve_rename("G036247", &r).unwrap();
        assert_eq!(rc.resolved, "DLR.TO");
        assert_eq!(rc.chain_repr, "G036247 AKA DLR.U.TO AKA DLR.TO");
    }

    #[test]
    fn test_resolve_rename_breaks_cycle() {
        // A -> B -> A should stop after one full traversal of the cycle
        // rather than looping forever.
        let r = renames(&[("A", "B"), ("B", "A")]);
        let rc = resolve_rename("A", &r).unwrap();
        // Resolution stops at B (the last unvisited hop) before cycling
        // back to A.
        assert_eq!(rc.resolved, "B");
        assert_eq!(rc.chain_repr, "A AKA B");
    }

    #[test]
    fn test_apply_security_rename_chain_extends_memo() {
        let r = renames(&[("G036247", "DLR.U.TO"), ("DLR.U.TO", "DLR.TO")]);

        let mut tx = CsvTx::default();
        tx.security = Some("G036247".into());
        tx.memo = Some("original".into());

        apply_security_rename(&mut tx, &r);

        assert_eq!(tx.security.as_deref(), Some("DLR.TO"));
        assert_eq!(
            tx.memo.as_deref(),
            Some("original ; G036247 AKA DLR.U.TO AKA DLR.TO")
        );
    }

    #[test]
    fn test_apply_security_rename_no_existing_memo() {
        let r = renames(&[("FOO", "FOO.TO")]);

        let mut tx = CsvTx::default();
        tx.security = Some("FOO".into());

        apply_security_rename(&mut tx, &r);

        assert_eq!(tx.security.as_deref(), Some("FOO.TO"));
        assert_eq!(tx.memo.as_deref(), Some("FOO AKA FOO.TO"));
    }

    #[test]
    fn test_apply_security_rename_empty_memo() {
        let r = renames(&[("FOO", "FOO.TO")]);

        let mut tx = CsvTx::default();
        tx.security = Some("FOO".into());
        tx.memo = Some(String::new());

        apply_security_rename(&mut tx, &r);

        assert_eq!(tx.memo.as_deref(), Some("FOO AKA FOO.TO"));
    }

    #[test]
    fn test_apply_security_rename_no_match_unchanged() {
        let r = renames(&[("FOO", "FOO.TO")]);

        let mut tx = CsvTx::default();
        tx.security = Some("BAR".into());
        tx.memo = Some("original".into());

        apply_security_rename(&mut tx, &r);

        assert_eq!(tx.security.as_deref(), Some("BAR"));
        assert_eq!(tx.memo.as_deref(), Some("original"));
    }

    #[test]
    fn test_apply_security_rename_none_security_noop() {
        let r = renames(&[("FOO", "FOO.TO")]);

        let mut tx = CsvTx::default();

        apply_security_rename(&mut tx, &r);

        assert!(tx.security.is_none());
        assert!(tx.memo.is_none());
    }
}
