use std::collections::HashMap;

use crate::portfolio::model::tx::CsvTx;

pub fn symbol_rename_memo_suffix(from: &str) -> String {
    format!("AKA {from}")
}

/// Apply a symbol rename to a single `CsvTx`, updating `security` and `memo`.
///
/// Looks up `tx.security` in `renames`; if found, renames the security and
/// appends the AKA suffix to the memo.  No-op if `tx.security` is `None` or
/// not present in the map.
pub fn apply_security_rename(tx: &mut CsvTx, renames: &HashMap<String, String>) {
    if let Some(sec) = tx.security.take() {
        if let Some(to) = renames.get(&sec) {
            let suffix = symbol_rename_memo_suffix(&sec);
            match &mut tx.memo {
                Some(m) if !m.is_empty() => {
                    m.push_str(" ; ");
                    m.push_str(&suffix);
                }
                Some(m) => *m = suffix,
                None => tx.memo = Some(suffix),
            }
            tx.security = Some(to.clone());
        } else {
            tx.security = Some(sec);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portfolio::model::tx::CsvTx;

    #[test]
    fn test_symbol_rename_memo_suffix() {
        assert_eq!(symbol_rename_memo_suffix("FOO"), "AKA FOO");
        assert_eq!(symbol_rename_memo_suffix("XEQT"), "AKA XEQT");
    }

    #[test]
    fn test_apply_security_rename_renames_and_extends_memo() {
        let renames: HashMap<String, String> =
            [("FOO".into(), "FOO.TO".into())].into_iter().collect();

        let mut tx = CsvTx::default();
        tx.security = Some("FOO".into());
        tx.memo = Some("original".into());

        apply_security_rename(&mut tx, &renames);

        assert_eq!(tx.security.as_deref(), Some("FOO.TO"));
        assert_eq!(tx.memo.as_deref(), Some("original ; AKA FOO"));
    }

    #[test]
    fn test_apply_security_rename_no_existing_memo() {
        let renames: HashMap<String, String> =
            [("FOO".into(), "FOO.TO".into())].into_iter().collect();

        let mut tx = CsvTx::default();
        tx.security = Some("FOO".into());

        apply_security_rename(&mut tx, &renames);

        assert_eq!(tx.security.as_deref(), Some("FOO.TO"));
        assert_eq!(tx.memo.as_deref(), Some("AKA FOO"));
    }

    #[test]
    fn test_apply_security_rename_empty_memo() {
        let renames: HashMap<String, String> =
            [("FOO".into(), "FOO.TO".into())].into_iter().collect();

        let mut tx = CsvTx::default();
        tx.security = Some("FOO".into());
        tx.memo = Some(String::new());

        apply_security_rename(&mut tx, &renames);

        assert_eq!(tx.memo.as_deref(), Some("AKA FOO"));
    }

    #[test]
    fn test_apply_security_rename_no_match_unchanged() {
        let renames: HashMap<String, String> =
            [("FOO".into(), "FOO.TO".into())].into_iter().collect();

        let mut tx = CsvTx::default();
        tx.security = Some("BAR".into());
        tx.memo = Some("original".into());

        apply_security_rename(&mut tx, &renames);

        assert_eq!(tx.security.as_deref(), Some("BAR"));
        assert_eq!(tx.memo.as_deref(), Some("original"));
    }

    #[test]
    fn test_apply_security_rename_none_security_noop() {
        let renames: HashMap<String, String> =
            [("FOO".into(), "FOO.TO".into())].into_iter().collect();

        let mut tx = CsvTx::default();

        apply_security_rename(&mut tx, &renames);

        assert!(tx.security.is_none());
        assert!(tx.memo.is_none());
    }
}
