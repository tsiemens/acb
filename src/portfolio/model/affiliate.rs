use std::{collections::HashMap, sync::{Arc, Mutex, MutexGuard}};
use lazy_static::lazy_static;
use regex::Regex;

#[derive(PartialEq, Eq, Debug)]
struct AffiliateData {
    id: String,
    name: String,
    registered: bool,
}

lazy_static! {
    static ref REGISTERED_RE: Regex = Regex::new(r"\([rR]\)").unwrap();
    static ref EXTRA_SPACE_RE: Regex = Regex::new(r"  +").unwrap();
}

impl AffiliateData {
    fn from_strep(s: &str) -> AffiliateData {
        let registered = REGISTERED_RE.is_match(s);
        let mut pretty_name: String = s.to_string();
        if registered {
            pretty_name = REGISTERED_RE.replace_all(&pretty_name, " ").to_string();
        }
        pretty_name = EXTRA_SPACE_RE.replace_all(&pretty_name, " ")
            .trim().to_string();
        if pretty_name.is_empty() {
            pretty_name = "Default".to_string();
        }
        let mut id = pretty_name.to_lowercase();
        if registered {
            id += " (R)";
            pretty_name += " (R)";
        }

        AffiliateData{id: id, name: pretty_name, registered: registered}
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Affiliate(Arc<AffiliateData>);

impl Affiliate {
    fn new(d: AffiliateData) -> Self {
        Affiliate(Arc::new(d))
    }

    pub fn from_strep(s: &str) -> Affiliate {
        AffiliateDedupTable::global_table().deduped_affiliate(s)
    }

    pub fn default() -> Affiliate {
        Affiliate::from_strep("")
    }

    pub fn default_registered() -> Affiliate {
        Affiliate::from_strep("(R)")
    }

    pub fn id(&self) -> &str {
        self.0.id.as_str()
    }
    pub fn name(&self) -> &str {
        self.0.name.as_str()
    }
    pub fn registered(&self) -> bool {
        self.0.registered
    }
    pub fn is_default(&self) -> bool {
        self.id().starts_with("default")
    }
}

impl std::hash::Hash for Affiliate {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.id.hash(state);
    }
}

pub struct AffiliateDedupTable {
    id_to_af: HashMap<String, Affiliate>
}

lazy_static! {
    static ref GLOBAL_AF_DEDUP_TABLE: Mutex<AffiliateDedupTable> = Mutex::new(
        AffiliateDedupTable::new());
}

impl AffiliateDedupTable {
    pub fn new() -> AffiliateDedupTable {
        AffiliateDedupTable{id_to_af: HashMap::new()}
    }

    pub fn global_table() -> MutexGuard<'static, AffiliateDedupTable> {
        GLOBAL_AF_DEDUP_TABLE.lock().unwrap()
    }

    pub fn deduped_affiliate(&mut self, strep: &str) -> Affiliate {
        let afd = AffiliateData::from_strep(strep);
        // let mut map = GLOBAL_AF_DEDUP_TABLE.id_to_af.lock().unwrap();
        match self.id_to_af.contains_key(afd.id.as_str()) {
            true => self.id_to_af.get(afd.id.as_str()).unwrap().clone(),
            false => {
                let af = Affiliate::new(afd);
                self.id_to_af.insert(af.id().to_string(), af.clone());
                af
            }
        }
    }

    pub fn must_get(&self, id: &str) -> &Affiliate {
        self.id_to_af.get(id).unwrap()
    }

    pub fn get_default_affiliate(&self) -> &Affiliate {
        self.must_get("default")
    }
}

#[cfg(test)]
mod tests {
    use super::AffiliateData;
    use super::Affiliate;
    use super::AffiliateDedupTable;

    #[test]
    fn test_affiliate() {
        let new = |s: &str| -> Affiliate {
            Affiliate::new(AffiliateData::from_strep(s))
        };

        let verify = |name: &str, exp_id: &str, exp_name: &str, exp_reg: bool| {
            let af = new(name);
            assert_eq!(exp_id, af.id());
            assert_eq!(exp_name, af.name());
            assert_eq!(exp_reg, af.registered());
        };

        assert_eq!(new(""), new(""));
        verify("", "default", "Default", false);
        verify("", "default", "Default", false);
        verify("  ", "default", "Default", false);
        verify("  default", "default", "default", false);
        verify("  Default", "default", "Default", false);

        verify(" (r) ", "default (R)", "Default (R)", true);
        verify("(R)", "default (R)", "Default (R)", true);
        verify("default(R)", "default (R)", "default (R)", true);
        verify("(R)Default", "default (R)", "Default (R)", true);
        verify("(R)Default(r)", "default (R)", "Default (R)", true);
        verify("Def(r)ault", "def ault (R)", "Def ault (R)", true);

        verify(" My Spouse ", "my spouse", "My Spouse", false);
        verify(" My     Spouse ", "my spouse", "My Spouse", false);
        verify(" My  (r)   Spouse ", "my spouse (R)", "My Spouse (R)", true);

        assert!(new("").is_default());
        assert!(new("").is_default());
        assert!(new("Default").is_default());
        assert!(new("(R)Default").is_default());
        assert!(new("(R)XXX").is_default() == false);
        assert!(new("XXX").is_default() == false);
        assert!(new("Def(r)ault").is_default() == false);
    }

    #[test]
    fn test_affiliate_dedup_table() {
        let mut dt = AffiliateDedupTable::new();

        // Check basic deduping for one entry
        let af1 = dt.deduped_affiliate("");
        assert_eq!(Affiliate::from_strep("Default"), af1);
        let af2 = dt.deduped_affiliate("  Default  ");
        let af3 = dt.deduped_affiliate("default");
        assert_eq!(af1, af2);
        assert_eq!(af1, af3);

        // Check that a different entry dedupes differently
        let af4 = dt.deduped_affiliate("(R)");
        assert_ne!(af1, af4);

        // Check that the first entry is still retained in the dedup table
        assert_eq!(af1, dt.deduped_affiliate("default"));
    }
}