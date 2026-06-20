use lazy_static::lazy_static;
use regex::Regex;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

#[derive(PartialEq, Eq, Debug)]
struct AffiliateData {
    // Normalized unique id, e.g. "default", "spouse (R)",
    // "default [rsu xyz 2026-02-20]". Lower-cased, with the registered "(R)"
    // suffix and/or cost pool "[tag]" appended. ACB is keyed off this.
    // A fully-specified id might be "default (R) [tag desc]".
    id: String,
    // The base display name only: the cleaned name with original casing, but
    // WITHOUT the registered "(R)" marker and WITHOUT any cost pool "[tag]"
    // (e.g. "Default", "My Spouse"). Whitespace is collapsed/trimmed, and an
    // empty name becomes "Default". An affiliate and its cost pools share this
    // base name (they differ only by tag), so it groups them together; see
    // `base_name_normalized`.
    name_base: String,
    // Full display name / canonical strep (round-trips through `from_strep`):
    // name_base, plus " (R)" if registered, plus " [tag]" if a cost pool.
    name: String,
    registered: bool,
    // The "cost pool" tag isolates an affiliate's ACB from the rest of its base
    // affiliate's lots (see the type docs on `Affiliate`). None for ordinary
    // affiliates.
    cost_pool_tag: Option<String>,
}

const GLOBAL_AF_ID: &str = "__global__";

lazy_static! {
    static ref REGISTERED_RE: Regex = Regex::new(r"\([rR]\)").unwrap();
    static ref EXTRA_SPACE_RE: Regex = Regex::new(r"  +").unwrap();
    // Matches a cost pool marker like `[RSU XYZ 2026-02-20]`. The capture group
    // is the cost pool tag. Everything outside the marker is the base affiliate.
    static ref COST_POOL_RE: Regex = Regex::new(r"\[([^\]]*)\]").unwrap();
}

impl AffiliateData {
    fn from_base_name(name_base: &str, registered: bool) -> AffiliateData {
        AffiliateData::from_parts(name_base, registered, None)
    }

    /// Build from a base name, registered status, and an optional cost pool tag.
    /// The base name should NOT contain `(R)` or a `[...]` cost pool marker.
    /// An empty or whitespace-only `cost_pool_tag` is treated as no cost pool.
    fn from_parts(
        name_base: &str,
        registered: bool,
        cost_pool_tag: Option<&str>,
    ) -> AffiliateData {
        let mut pretty_name =
            EXTRA_SPACE_RE.replace_all(name_base, " ").trim().to_string();

        if pretty_name.is_empty() {
            pretty_name = "Default".to_string();
        }
        let name_base_cleaned = pretty_name.clone();

        // base_id/base_name are the id/name with no cost pool tag.
        let mut base_id = pretty_name.to_lowercase();
        let mut base_name = pretty_name;
        if registered {
            base_id += " (R)";
            base_name += " (R)";
        }

        // Normalize the tag, ignoring empty/whitespace-only tags.
        let cost_pool_tag: Option<String> = cost_pool_tag.and_then(|t| {
            let cleaned = EXTRA_SPACE_RE.replace_all(t, " ").trim().to_string();
            if cleaned.is_empty() {
                None
            } else {
                Some(cleaned)
            }
        });

        let (id, name) = match &cost_pool_tag {
            Some(tag) => (
                format!("{} [{}]", base_id, tag.to_lowercase()),
                format!("{} [{}]", base_name, tag),
            ),
            None => (base_id, base_name),
        };

        AffiliateData {
            id,
            name_base: name_base_cleaned,
            name,
            registered,
            cost_pool_tag,
        }
    }

    fn from_strep(s: &str) -> AffiliateData {
        // Extract an optional cost pool marker `[<tag>]`. Everything outside the
        // marker is the base affiliate. The first marker wins if multiple are
        // somehow present.
        let (base_strep, cost_pool_tag): (String, Option<String>) =
            match COST_POOL_RE.captures(s) {
                Some(caps) => {
                    let tag = caps.get(1).unwrap().as_str().to_string();
                    let whole = caps.get(0).unwrap();
                    let mut base = s.to_string();
                    // Replace with a space so adjacent tokens don't merge; the
                    // extra space is collapsed/trimmed in from_parts.
                    base.replace_range(whole.start()..whole.end(), " ");
                    (base, Some(tag))
                }
                None => (s.to_string(), None),
            };

        let registered = REGISTERED_RE.is_match(&base_strep);
        let mut pretty_name = base_strep;
        if registered {
            pretty_name = REGISTERED_RE.replace_all(&pretty_name, " ").to_string();
        }
        AffiliateData::from_parts(&pretty_name, registered, cost_pool_tag.as_deref())
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
///
/// ## Cost pools
///
/// An affiliate may additionally carry a "cost pool" tag, encoded in its strep as
/// a bracketed marker `[<tag>]` (e.g. `Default [7(1.31) - RSU XYZ 2026-02-20]`).
/// Like the `(R)` registered marker, it may appear anywhere in the strep, not only
/// at the end. This is used to isolate a subset of a security's lots into their own
/// ACB while still belonging to the same person/account. This matters for ITA
/// subsection 7(1.31) "benefit" sales, where the cost base must be averaged
/// separately, yet superficial loss rules still apply across the lots. Modelling
/// each such pool as a distinct Affiliate gives it a self-contained ACB (ACB is
/// keyed per-affiliate id), while superficial-loss detection — which already
/// operates across all affiliates of a security — keeps treating them as one
/// inventory.
///
/// A cost pool shares its `name_base` with the ordinary affiliate it belongs to,
/// so `base_name_normalized` (and the web GUI's equivalent base-name grouping)
/// naturally folds cost pools back under their parent affiliate in filters rather
/// than listing each separately.
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

    /// The cost pool tag isolating this affiliate's ACB from the rest of its
    /// base affiliate's lots, or None for an ordinary affiliate. See the type
    /// docs.
    pub fn cost_pool_tag(&self) -> Option<&str> {
        self.0.cost_pool_tag.as_deref()
    }

    /// Whether this affiliate is an isolated cost pool (has a cost pool tag).
    pub fn is_cost_pool(&self) -> bool {
        self.0.cost_pool_tag.is_some()
    }

    /// Returns the cost-pool variant of this affiliate: the same base name and
    /// registered status, but carrying the given cost pool `tag` so it gets a
    /// self-contained ACB. See the type docs on cost pools.
    ///
    /// Any existing cost pool tag on `self` is replaced. An empty or
    /// whitespace-only `tag` yields the plain (untagged) base affiliate.
    pub fn with_cost_pool_tag(&self, tag: &str) -> Affiliate {
        let afd = AffiliateData::from_parts(
            &self.0.name_base,
            self.0.registered,
            Some(tag),
        );
        AffiliateDedupTable::global_table().deduped_affiliate_from_afd(afd)
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

    #[test]
    fn test_affiliate_cost_pool() {
        let new =
            |s: &str| -> Affiliate { Affiliate::new(AffiliateData::from_strep(s)) };

        let verify = |strep: &str,
                      exp_id: &str,
                      exp_name: &str,
                      exp_reg: bool,
                      exp_tag: Option<&str>| {
            let af = new(strep);
            assert_eq!(exp_id, af.id(), "id for {strep:?}");
            assert_eq!(exp_name, af.name(), "name for {strep:?}");
            assert_eq!(exp_reg, af.registered(), "registered for {strep:?}");
            assert_eq!(exp_tag, af.cost_pool_tag(), "tag for {strep:?}");
            assert_eq!(
                exp_tag.is_some(),
                af.is_cost_pool(),
                "is_cost_pool {strep:?}"
            );
        };

        // Default base, with a cost pool tag (no explicit base name).
        verify(
            "[RSU XYZ 2026-02-20]",
            "default [rsu xyz 2026-02-20]",
            "Default [RSU XYZ 2026-02-20]",
            false,
            Some("RSU XYZ 2026-02-20"),
        );

        // Named base, with a cost pool tag.
        verify(
            "Default [RSU XYZ 2026-02-20]",
            "default [rsu xyz 2026-02-20]",
            "Default [RSU XYZ 2026-02-20]",
            false,
            Some("RSU XYZ 2026-02-20"),
        );

        // Registered base, with a cost pool tag.
        verify(
            "Spouse (R) [ESO 2025-01-02]",
            "spouse (R) [eso 2025-01-02]",
            "Spouse (R) [ESO 2025-01-02]",
            true,
            Some("ESO 2025-01-02"),
        );

        // Empty/whitespace tags are ignored (treated as an ordinary affiliate).
        verify("Spouse []", "spouse", "Spouse", false, None);
        verify("Spouse [   ]", "spouse", "Spouse", false, None);

        // The marker may appear anywhere in the strep, not only at the end
        // (same as the `(R)` registered marker).
        assert_eq!(new("[RSU A] Spouse"), new("Spouse [RSU A]"));
        assert_eq!(new("Spouse [RSU A] (R)"), new("Spouse (R) [RSU A]"));

        // A `(r)` *inside* the tag must NOT register the affiliate: the marker
        // is extracted before the registered check, so the `(r)` stays part of
        // the tag text.
        verify(
            "Default [some tag (r)]",
            "default [some tag (r)]",
            "Default [some tag (r)]",
            false,
            Some("some tag (r)"),
        );
        // ...while a `(R)` outside the tag still registers the base, leaving an
        // in-tag `(r)` untouched.
        verify(
            "Default (R) [foo (r)]",
            "default (R) [foo (r)]",
            "Default (R) [foo (r)]",
            true,
            Some("foo (r)"),
        );

        let plain = new("Spouse");
        assert!(!plain.is_cost_pool());
        assert_eq!(plain.cost_pool_tag(), None);

        // name() round-trips through from_strep for cost pools.
        for s in [
            "Default [RSU XYZ 2026-02-20]",
            "Spouse (R) [ESO 2025-01-02]",
            "[RSU XYZ 2026-02-20]",
            "Default [some tag (r)]",
            "Default (R) [foo (r)]",
        ] {
            let af = new(s);
            assert_eq!(af, new(af.name()), "round-trip for {s:?}");
        }

        // Two different tags on the same base are distinct affiliates (separate
        // ACB), but share a base name so they cluster together in filters.
        let pool_a = new("Default [RSU A]");
        let pool_b = new("Default [RSU B]");
        let base = new("Default");
        assert_ne!(pool_a, pool_b);
        assert_ne!(pool_a, base);
        // base_name_normalized() folds cost pools into their base, which is how
        // the GUI groups them (see also the web `affiliateBaseName`).
        assert_eq!(pool_a.base_name_normalized(), base.base_name_normalized());
        assert_eq!(pool_a.base_name_normalized(), pool_b.base_name_normalized());
        assert_ne!(
            pool_a.base_name_normalized(),
            new("Spouse [RSU A]").base_name_normalized()
        );
    }

    #[test]
    fn test_with_cost_pool_tag() {
        // Derives the tagged variant from a plain base affiliate.
        let base = Affiliate::from_strep("Spouse");
        let pool = base.with_cost_pool_tag("RSU A");
        assert_eq!(pool, Affiliate::from_strep("Spouse [RSU A]"));
        assert_eq!(pool.cost_pool_tag(), Some("RSU A"));

        // Preserves registered status.
        let reg = Affiliate::from_strep("Spouse (R)");
        let reg_pool = reg.with_cost_pool_tag("ESO 2025-01-02");
        assert_eq!(
            reg_pool,
            Affiliate::from_strep("Spouse (R) [ESO 2025-01-02]")
        );
        assert!(reg_pool.registered());

        // Replaces an existing tag rather than nesting it.
        assert_eq!(
            pool.with_cost_pool_tag("RSU B"),
            base.with_cost_pool_tag("RSU B")
        );

        // An empty/whitespace tag yields the plain base affiliate.
        assert_eq!(base.with_cost_pool_tag(""), base);
        assert_eq!(base.with_cost_pool_tag("   "), base);
    }
}
