use lazy_static::lazy_static;
use regex::Regex;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

#[derive(PartialEq, Eq, Debug)]
struct AffiliateData {
    id: String,
    name_base: String,
    name: String,
    registered: bool,
}

const GLOBAL_AF_ID: &str = "__global__";

lazy_static! {
    static ref REGISTERED_RE: Regex = Regex::new(r"\([rR]\)").unwrap();
    static ref EXTRA_SPACE_RE: Regex = Regex::new(r"  +").unwrap();
}

impl AffiliateData {
    fn from_base_name(name_base: &str, registered: bool) -> AffiliateData {
        let mut pretty_name =
            EXTRA_SPACE_RE.replace_all(&name_base, " ").trim().to_string();

        if pretty_name.is_empty() {
            pretty_name = "Default".to_string();
        }
        let mut id = pretty_name.to_lowercase();
        let name_base_cleaned = pretty_name.clone();
        if registered {
            id += " (R)";
            pretty_name += " (R)";
        }

        AffiliateData {
            id: id,
            name_base: name_base_cleaned,
            name: pretty_name,
            registered: registered,
        }
    }

    fn from_strep(s: &str) -> AffiliateData {
        let registered = REGISTERED_RE.is_match(s);
        let mut pretty_name: String = s.to_string();
        if registered {
            pretty_name = REGISTERED_RE.replace_all(&pretty_name, " ").to_string();
        }
        AffiliateData::from_base_name(&pretty_name, registered)
    }
}

/// An Affiliate is a person or entity associated with transactions, such as you,
/// your spouse, a company, etc. Each 'base' affiliate, at least those associated
/// with a real person, can also have a registered and non-registered variant.
///
/// We represent each Affiliate with a 'base' name (e.g. "Default", "Spouse") etc,
/// and a registered status. The "name" here is a display name, and the id is a
/// normalized version of this. It will include "(R)" if registered.
///
/// Default is a special reserved name, normally used for yourself, or the
/// primary person managing the portfolio. Though it need not be used.
///
/// Storage efficiency isn't a high concern here, since we deduplicate AffiliateData,
/// which means the first time an affiliate is encountered, we store the full name,
/// but subsequent times, we'll just pick up the previous with the same id.
/// As a side-effect, this means that capitalization differences will be resolved by
/// the first Affiliate to be deduplicated.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Affiliate(Arc<AffiliateData>);

impl Affiliate {
    fn new(d: AffiliateData) -> Self {
        Affiliate(Arc::new(d))
    }

    pub fn from_strep(s: &str) -> Affiliate {
        AffiliateDedupTable::global_table().deduped_affiliate(s)
    }

    /// Create an affiliate with the given base name and registered status.
    /// The name should NOT contain `(R)` — pass `registered: true` instead.
    pub fn from_base_name(name: &str, registered: bool) -> Affiliate {
        let afd = AffiliateData::from_base_name(name, registered);
        AffiliateDedupTable::global_table().deduped_affiliate_from_afd(afd)
    }

    pub fn default() -> Affiliate {
        Affiliate::from_strep("")
    }

    pub fn default_registered() -> Affiliate {
        Affiliate::from_strep("(R)")
    }

    pub fn global() -> Affiliate {
        Affiliate::from_strep(GLOBAL_AF_ID)
    }

    pub fn id(&self) -> &str {
        self.0.id.as_str()
    }
    pub fn name(&self) -> &str {
        self.0.name.as_str()
    }
    pub fn base_name_normalized(&self) -> String {
        self.0.name_base.to_lowercase()
    }
    pub fn registered(&self) -> bool {
        self.0.registered
    }
    pub fn is_default(&self) -> bool {
        self.id().starts_with("default")
    }

    // Special transactions (such as splits) may specify the global affiliate,
    // which indicates it applies across all affiliates.
    pub fn is_global(&self) -> bool {
        self.id() == GLOBAL_AF_ID
    }
}

impl std::hash::Hash for Affiliate {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.id.hash(state);
    }
}

pub struct AffiliateDedupTable {
    id_to_af: HashMap<String, Affiliate>,
}

lazy_static! {
    static ref GLOBAL_AF_DEDUP_TABLE: Mutex<AffiliateDedupTable> =
        Mutex::new(AffiliateDedupTable::new());
}

impl AffiliateDedupTable {
    pub fn new() -> AffiliateDedupTable {
        AffiliateDedupTable {
            id_to_af: HashMap::new(),
        }
    }

    pub fn global_table() -> MutexGuard<'static, AffiliateDedupTable> {
        GLOBAL_AF_DEDUP_TABLE.lock().unwrap()
    }

    pub(self) fn deduped_affiliate_from_afd(
        &mut self,
        afd: AffiliateData,
    ) -> Affiliate {
        match self.id_to_af.contains_key(afd.id.as_str()) {
            true => self.id_to_af.get(afd.id.as_str()).unwrap().clone(),
            false => {
                let af = Affiliate::new(afd);
                self.id_to_af.insert(af.id().to_string(), af.clone());
                af
            }
        }
    }

    pub fn deduped_affiliate(&mut self, strep: &str) -> Affiliate {
        let afd = AffiliateData::from_strep(strep);
        self.deduped_affiliate_from_afd(afd)
    }

    pub fn must_get(&self, id: &str) -> &Affiliate {
        self.id_to_af.get(id).unwrap()
    }

    pub fn get_default_affiliate(&self) -> &Affiliate {
        self.must_get("default")
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct AffiliateFilter {
    pub non_registered: Affiliate,
    pub registered: Affiliate,
}

impl AffiliateFilter {
    pub fn new(non_registered_name: &str) -> Self {
        let non_registered = Affiliate::from_base_name(non_registered_name, false);
        let registered = Affiliate::from_base_name(non_registered_name, true);
        Self {
            non_registered,
            registered,
        }
    }

    pub fn affiliates(&self) -> Vec<Affiliate> {
        vec![self.non_registered.clone(), self.registered.clone()]
    }

    pub fn matches(&self, af: &Affiliate) -> bool {
        af == &self.non_registered || af == &self.registered
    }
}

#[cfg(test)]
mod tests {
    use super::Affiliate;
    use super::AffiliateData;
    use super::AffiliateDedupTable;

    #[test]
    fn test_affiliate() {
        let new =
            |s: &str| -> Affiliate { Affiliate::new(AffiliateData::from_strep(s)) };

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

        // This is reserved.
        verify(" __global__ ", "__global__", "__global__", false);
        // This is not reserved.
        verify(" Global ", "global", "Global", false);

        assert!(new("").is_default());
        assert!(new("").is_default());
        assert!(new("Default").is_default());
        assert!(new("(R)Default").is_default());
        assert!(new("(R)XXX").is_default() == false);
        assert!(new("XXX").is_default() == false);
        assert!(new("Def(r)ault").is_default() == false);

        assert!(new("__global__").is_global() == true);
        assert!(Affiliate::global().is_global() == true);
        assert!(new("Global").is_global() == false);
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

    #[test]
    fn test_from_name() {
        let af = Affiliate::from_base_name("Spouse", false);
        assert_eq!(af, Affiliate::from_strep("Spouse"));
        assert!(!af.registered());

        let af = Affiliate::from_base_name("Spouse", true);
        assert_eq!(af, Affiliate::from_strep("Spouse (R)"));
        assert!(af.registered());

        // Empty name becomes Default
        let af = Affiliate::from_base_name("", false);
        assert_eq!(af, Affiliate::default());

        let af = Affiliate::from_base_name("", true);
        assert_eq!(af, Affiliate::default_registered());
    }
}
