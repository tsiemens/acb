use std::{collections::HashMap, rc::Rc};

use crate::{
    portfolio::{Affiliate, PortfolioSecurityStatus},
    util::decimal::GreaterEqualZeroDecimal
};

/// Tracks the most recent PortfolioSecurityStatus for each Affiliate in
/// the portfolio for a single security, as well as any aggregate
/// information across all affiliates.
///
/// The user of this type is intended to repeatedly call set_patest_post_status,
/// which will update it to the latest state.
pub struct AffiliatePortfolioSecurityStatuses {
    last_post_status_for_affiliate: HashMap<Affiliate, Rc<PortfolioSecurityStatus>>,
    security: String,
    latest_all_affiliates_share_balance: GreaterEqualZeroDecimal,
    latest_affiliate: Affiliate,
}

impl AffiliatePortfolioSecurityStatuses {
    /// Create a new AffiliatePortfolioSecurityStatuses.
    /// Panics if initial_default_aff_status's share_balance and all_affiliate_share_balance are
    /// not equal. This is because if they were not equal, it would imply that some
    /// other transaction/status was before this, so it would not actually be the initial.
    pub fn new(security: String, initial_default_aff_status: Option<Rc<PortfolioSecurityStatus>>)
        -> AffiliatePortfolioSecurityStatuses {
        let mut s = AffiliatePortfolioSecurityStatuses{
            last_post_status_for_affiliate: HashMap::new(),
            security: security,
            latest_all_affiliates_share_balance: GreaterEqualZeroDecimal::zero(),
            latest_affiliate: Affiliate::default(),
        };

	    // Initial status only applies to the default affiliate
        if let Some(init_status) = initial_default_aff_status {
            // Just assert. The caller will have to verify.
            assert_eq!(init_status.share_balance, init_status.all_affiliate_share_balance);
            s.set_latest_post_status(&Affiliate::default(), init_status);
        }

        s
    }

    pub fn get_latest_post_status_for_affiliate(&self, af: &Affiliate) -> Option<&Rc<PortfolioSecurityStatus>> {
        self.last_post_status_for_affiliate.get(af)
    }

    fn make_default_portfolio_security_status(&self, af: &Affiliate) -> PortfolioSecurityStatus {
        let zero = GreaterEqualZeroDecimal::zero();
        PortfolioSecurityStatus{
            security: self.security.clone(),
            share_balance: zero,
            all_affiliate_share_balance: zero,
            total_acb: if af.registered() { None } else { Some(zero) },
        }
    }

    pub fn get_latest_post_status(&self) -> Rc<PortfolioSecurityStatus> {
        match self.get_latest_post_status_for_affiliate(&self.latest_affiliate) {
            Some(s) => s.clone(),
            None => Rc::new(self.make_default_portfolio_security_status(&self.latest_affiliate)),
        }
    }

    pub fn set_latest_post_status(&mut self,
                                  af: &Affiliate, v: Rc<PortfolioSecurityStatus>) {
        let last_share_balance = match self.last_post_status_for_affiliate.get(&af) {
            Some(status) => status.share_balance,
            None => GreaterEqualZeroDecimal::zero(),
        };
        let expected_all_share_bal =
            *v.share_balance + *self.latest_all_affiliates_share_balance - *last_share_balance;

        assert_eq!(af.registered(), v.total_acb.is_none(),
            "In security {}, af {}, total_acb has bad value ({:#?})",
            self.security, af.name(), v.total_acb);
        assert_eq!(*v.all_affiliate_share_balance, expected_all_share_bal,
            "In security {}, af {}, v.all_affiliate_share_balance ({}) != expected_all_share_bal ({}) \
            (*v.share_balance ({}) + *self.latest_all_affiliates_share_balance ({}) - *last_share_balance ({})",
            self.security, af.name(), v.all_affiliate_share_balance, expected_all_share_bal,
            *v.share_balance, *self.latest_all_affiliates_share_balance, *last_share_balance);

        self.last_post_status_for_affiliate.insert(af.clone(), v.clone());
        self.latest_all_affiliates_share_balance = v.all_affiliate_share_balance;
        self.latest_affiliate = af.clone();
    }

    pub fn get_next_pre_status(&self, af: &Affiliate) -> Rc<PortfolioSecurityStatus> {
        let last_status_for_af = match self.get_latest_post_status_for_affiliate(af) {
            Some(s) => s.clone(),
            None => Rc::new(self.make_default_portfolio_security_status(af)),
        };

        if last_status_for_af.all_affiliate_share_balance == self.latest_all_affiliates_share_balance {
            // The last status was for the same affiliate, so we can just
            // return the last status.
            last_status_for_af
        } else {
            // The actual last status was for a different affiliate, so the total
            // share balance on the last status of `af` isn't the most current.
            // Create a new copy with the updated global share balance.
            let mut next_pre_status: PortfolioSecurityStatus = (*last_status_for_af).clone();
            next_pre_status.all_affiliate_share_balance = self.latest_all_affiliates_share_balance;
            Rc::new(next_pre_status)
        }
    }
}

#[cfg(test)]
pub mod testlib {
    use std::rc::Rc;

    use crate::portfolio::testlib::{default_sec, MAGIC_DEFAULT_GEZ};
    use crate::{portfolio::PortfolioSecurityStatus, util::decimal::GreaterEqualZeroDecimal};


    #[derive(Debug)]
    pub struct TPSS {
        pub sec: String,
        pub shares: GreaterEqualZeroDecimal,
        pub all_shares: GreaterEqualZeroDecimal,
        pub total_acb: Option<GreaterEqualZeroDecimal>,
        pub acb_per_sh: Option<GreaterEqualZeroDecimal>,
    }

    impl TPSS {
        pub fn x(&self) -> Rc<PortfolioSecurityStatus> {
            Rc::new(PortfolioSecurityStatus{
                security: self.sec.clone(),
                share_balance: self.shares,
                all_affiliate_share_balance: if self.all_shares != *MAGIC_DEFAULT_GEZ {
                    self.all_shares } else { self.shares },
                total_acb: if let Some(aps) = self.acb_per_sh {
                    Some(GreaterEqualZeroDecimal::try_from(*aps * *self.shares).unwrap())
                } else {
                    self.total_acb
                },
            })
        }

        pub fn d() -> Self {
            Self::default()
        }
    }

    impl Default for TPSS {
        fn default() -> Self {
            Self{
                sec: default_sec(),
                shares: GreaterEqualZeroDecimal::zero(),
                all_shares: *MAGIC_DEFAULT_GEZ,
                total_acb: None,
                acb_per_sh: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::{portfolio::{bookkeeping::testlib::TPSS, testlib::default_sec, Affiliate, PortfolioSecurityStatus}, util::decimal::GreaterEqualZeroDecimal};
    use crate::gezdec as gez;

    use super::AffiliatePortfolioSecurityStatuses;

    fn gez_zero() -> GreaterEqualZeroDecimal {
        GreaterEqualZeroDecimal::zero()
    }

    #[test]
    fn test_get_latest_for_affiliate_basic() {
        let default_af = Affiliate::default();
        let af_b = Affiliate::from_strep("B");

        // Case:
        // get_latest_post_status_for_affiliate("default");
        // get_latest_post_status_for_affiliate("B");
        let statuses = AffiliatePortfolioSecurityStatuses::new(default_sec(), None);
        let default_pss = statuses.get_latest_post_status_for_affiliate(&default_af);
        assert_eq!(default_pss, None);
        let b_pss = statuses.get_latest_post_status_for_affiliate(&af_b);
        assert_eq!(b_pss, None);

        // Case:
        // (initial default state)
        // get_latest_post_status_for_affiliate("default");
        // get_latest_post_status_for_affiliate("B");
        let non_zero_init_status = Rc::new(PortfolioSecurityStatus{
            security: default_sec(), share_balance: gez!(12),
            all_affiliate_share_balance: gez!(12),
            total_acb: Some(gez!(24)),
        });
        let statuses = AffiliatePortfolioSecurityStatuses::new(
            default_sec(), Some(non_zero_init_status.clone()));

        let default_pss = statuses.get_latest_post_status_for_affiliate(&default_af);
        assert_eq!(default_pss.unwrap().as_ref(), non_zero_init_status.as_ref());
        let b_pss = statuses.get_latest_post_status_for_affiliate(&af_b);
        assert_eq!(b_pss, None);
    }

    #[test]
    fn test_get_latest() {
        let af_b = Affiliate::from_strep("B");

        // Case:
        // get_latest_post_status()
        let statuses = AffiliatePortfolioSecurityStatuses::new(default_sec(), None);
        let latest = statuses.get_latest_post_status();

        assert_eq!(TPSS{shares: gez_zero(), total_acb: Some(gez_zero()), ..TPSS::default()}.x(), latest);

        // Case:
        // (init with default)
        // get_latest_post_status()
        let non_zero_init_status = Rc::new(PortfolioSecurityStatus{
            security: default_sec(), share_balance: gez!(12),
            all_affiliate_share_balance: gez!(12),
            total_acb: Some(gez!(24)),
        });
        let statuses = AffiliatePortfolioSecurityStatuses::new(
            default_sec(), Some(non_zero_init_status.clone()));
        let latest = statuses.get_latest_post_status();
        assert_eq!(TPSS{shares: gez!(12), total_acb: Some(gez!(24)), ..TPSS::default()}.x(), latest);

        // Case:
        // set_latest_post_status("B")
        // get_latest_post_status()
        let mut statuses = AffiliatePortfolioSecurityStatuses::new(default_sec(), None);
        statuses.set_latest_post_status(&af_b, TPSS{shares: gez!(2), total_acb: Some(gez!(4)), ..TPSS::default()}.x());
        let latest = statuses.get_latest_post_status();
        assert_eq!(TPSS{shares: gez!(2), total_acb: Some(gez!(4)), ..TPSS::default()}.x(), latest);

        // Case:
        // (init with default)
        // set_latest_post_status("B")
        // get_latest_post_status()
        let mut statuses = AffiliatePortfolioSecurityStatuses::new(
            default_sec(), Some(non_zero_init_status.clone()));
        statuses.set_latest_post_status(&af_b, TPSS{shares: gez!(2), all_shares: gez!(14), total_acb: Some(gez!(4)), ..TPSS::default()}.x());
        let latest = statuses.get_latest_post_status();
        assert_eq!(TPSS{shares: gez!(2), all_shares: gez!(14), total_acb: Some(gez!(4)), ..TPSS::default()}.x(), latest);
    }

    #[test]
    #[should_panic]
    fn test_panic_on_invalid_all_shares() {
        let af_b = Affiliate::from_strep("B");
        let non_zero_init_status = Rc::new(PortfolioSecurityStatus{
            security: default_sec(), share_balance: gez!(12),
            all_affiliate_share_balance: gez!(12),
            total_acb: Some(gez!(24)),
        });

        // Case:
        // (init with default)
        // set_latest_post_status("B") :
        //      invalid all share bal, since it's not equal to 2 +
        //      the old all_shares_balance
        let mut statuses = AffiliatePortfolioSecurityStatuses::new(
            default_sec(), Some(non_zero_init_status.clone()));
        statuses.set_latest_post_status(&af_b, TPSS{shares: gez!(2), all_shares: gez!(2), total_acb: Some(gez!(4)), ..TPSS::d()}.x());
    }

    #[test]
    fn test_get_next_pre_get_latest() {
        let default_af = Affiliate::default();
        let af_b = Affiliate::from_strep("B");

        // Case:
        // set_latest_post_status("B")
        // get_next_pre_status("Default")
        // get_latest_post_status()
        let mut statuses = AffiliatePortfolioSecurityStatuses::new(default_sec(), None);
        statuses.set_latest_post_status(&af_b, TPSS{shares: gez!(2), all_shares: gez!(2), total_acb: Some(gez!(4)), ..TPSS::default()}.x());
        let default_status = statuses.get_next_pre_status(&default_af);
        assert_eq!(TPSS{shares: gez_zero(), all_shares: gez!(2), total_acb: Some(gez_zero()), ..TPSS::default()}.x(), default_status);
        let latest = statuses.get_latest_post_status();
        assert_eq!(TPSS{shares: gez!(2), all_shares: gez!(2), total_acb: Some(gez!(4)), ..TPSS::default()}.x(), latest);

        // Case:
        // (init with default)
        // set_latest_post_status("B")
        // get_next_pre_status("Default")
        // get_latest_post_status()
        let non_zero_init_status = Rc::new(PortfolioSecurityStatus{
            security: default_sec(), share_balance: gez!(12),
            all_affiliate_share_balance: gez!(12),
            total_acb: Some(gez!(24)),
        });
        let mut statuses = AffiliatePortfolioSecurityStatuses::new(
            default_sec(), Some(non_zero_init_status.clone()));
        statuses.set_latest_post_status(&af_b, TPSS{shares: gez!(2), all_shares: gez!(14), total_acb: Some(gez!(4)), ..TPSS::default()}.x());
        let default_status = statuses.get_next_pre_status(&default_af);
        assert_eq!(TPSS{shares: gez!(12), all_shares: gez!(14), total_acb: Some(gez!(24)), ..TPSS::default()}.x(), default_status);
        let latest = statuses.get_latest_post_status();
        assert_eq!(TPSS{shares: gez!(2), all_shares: gez!(14), total_acb: Some(gez!(4)), ..TPSS::default()}.x(), latest);

    }

    #[test]
    fn test_full_use_case() {
        let default_af = Affiliate::default();
        let af_b = Affiliate::from_strep("B");

        // Case:
        // get_next_pre_status("Default")
        // Get*
        // set_latest_post_status("Default")
        //
        // get_next_pre_status("Default")
        // Get*
        // set_latest_post_status("Default")
        //
        // get_next_pre_status("B")
        // Get*
        // set_latest_post_status("B")
        //
        // get_next_pre_status("B")
        // Get*
        // set_latest_post_status("B")
        //
        // get_next_pre_status("Default")
        // Get*
        // set_latest_post_status("Default")

        // Buy 2 default
        let mut statuses = AffiliatePortfolioSecurityStatuses::new(default_sec(), None);
        let next_pre = statuses.get_next_pre_status(&default_af);
        assert_eq!(TPSS{shares: gez_zero(), all_shares: gez_zero(), total_acb: Some(gez_zero()), ..TPSS::default()}.x(), next_pre);
        let latest_post = statuses.get_latest_post_status();
        assert_eq!(TPSS{shares: gez_zero(), all_shares: gez_zero(), total_acb: Some(gez_zero()), ..TPSS::default()}.x(), latest_post);
        assert_eq!(statuses.get_latest_post_status_for_affiliate(&default_af), None);
        assert_eq!(statuses.get_latest_post_status_for_affiliate(&af_b), None);
        statuses.set_latest_post_status(&default_af, TPSS{shares: gez!(2), total_acb: Some(gez!(4)), ..TPSS::default()}.x());

        // Buy 1 default
        let next_pre = statuses.get_next_pre_status(&default_af);
        assert_eq!(TPSS{shares: gez!(2), total_acb: Some(gez!(4)), ..TPSS::default()}.x(), next_pre);
        let latest_post = statuses.get_latest_post_status();
        assert_eq!(TPSS{shares: gez!(2), all_shares: gez!(2), total_acb: Some(gez!(4)), ..TPSS::default()}.x(), latest_post);
        let latest_def = statuses.get_latest_post_status_for_affiliate(&default_af).unwrap();
        assert_eq!(TPSS{shares: gez!(2), all_shares: gez!(2), total_acb: Some(gez!(4)), ..TPSS::default()}.x(), latest_def.clone());
        assert_eq!(statuses.get_latest_post_status_for_affiliate(&af_b), None);
        statuses.set_latest_post_status(&default_af, TPSS{shares: gez!(3), total_acb: Some(gez!(6)), ..TPSS::default()}.x());

        // Buy 12 B
        let next_pre = statuses.get_next_pre_status(&af_b);
        assert_eq!(TPSS{shares: gez_zero(), all_shares: gez!(3), total_acb: Some(gez_zero()), ..TPSS::default()}.x(), next_pre);
        let latest_post = statuses.get_latest_post_status();
        assert_eq!(TPSS{shares: gez!(3), all_shares: gez!(3), total_acb: Some(gez!(6)), ..TPSS::default()}.x(), latest_post);
        let latest_def = statuses.get_latest_post_status_for_affiliate(&default_af).unwrap();
        assert_eq!(TPSS{shares: gez!(3), all_shares: gez!(3), total_acb: Some(gez!(6)), ..TPSS::default()}.x(), latest_def.clone());
        assert_eq!(statuses.get_latest_post_status_for_affiliate(&af_b), None);
        statuses.set_latest_post_status(&af_b, TPSS{shares: gez!(12), all_shares: gez!(15), total_acb: Some(gez!(24)), ..TPSS::default()}.x());

        // Sell 6 B
        let next_pre = statuses.get_next_pre_status(&af_b);
        assert_eq!(TPSS{shares: gez!(12), all_shares: gez!(15), total_acb: Some(gez!(24)), ..TPSS::default()}.x(), next_pre);
        let latest_post = statuses.get_latest_post_status();
        assert_eq!(TPSS{shares: gez!(12), all_shares: gez!(15), total_acb: Some(gez!(24)), ..TPSS::default()}.x(), latest_post);
        let latest_def = statuses.get_latest_post_status_for_affiliate(&default_af).unwrap();
        assert_eq!(TPSS{shares: gez!(3), all_shares: gez!(3), total_acb: Some(gez!(6)), ..TPSS::default()}.x(), latest_def.clone());
        let latest_b = statuses.get_latest_post_status_for_affiliate(&af_b).unwrap();
        assert_eq!(TPSS{shares: gez!(12), all_shares: gez!(15), total_acb: Some(gez!(24)), ..TPSS::default()}.x(), latest_b.clone());
        statuses.set_latest_post_status(&af_b, TPSS{shares: gez!(6), all_shares: gez!(9), total_acb: Some(gez!(12)), ..TPSS::default()}.x());

        // Buy 1 default
        let next_pre = statuses.get_next_pre_status(&default_af);
        assert_eq!(TPSS{shares: gez!(3), all_shares: gez!(9), total_acb: Some(gez!(6)), ..TPSS::default()}.x(), next_pre);
        let latest_post = statuses.get_latest_post_status();
        assert_eq!(TPSS{shares: gez!(6), all_shares: gez!(9), total_acb: Some(gez!(12)), ..TPSS::default()}.x(), latest_post);
        let latest_def = statuses.get_latest_post_status_for_affiliate(&default_af).unwrap();
        assert_eq!(TPSS{shares: gez!(3), all_shares: gez!(3), total_acb: Some(gez!(6)), ..TPSS::default()}.x(), latest_def.clone());
        let latest_b = statuses.get_latest_post_status_for_affiliate(&af_b).unwrap();
        assert_eq!(TPSS{shares: gez!(6), all_shares: gez!(9), total_acb: Some(gez!(12)), ..TPSS::default()}.x(), latest_b.clone());
        statuses.set_latest_post_status(&default_af, TPSS{shares: gez!(4), all_shares: gez!(10), total_acb: Some(gez!(6)), ..TPSS::default()}.x());

    }

    #[test]
    fn test_registered() {
        let default_r_af = Affiliate::default_registered();

        let mut statuses = AffiliatePortfolioSecurityStatuses::new(default_sec(), None);

        // Case:
        // get_next_pre_status("(R)")
        let next_pre = statuses.get_next_pre_status(&default_r_af);
        assert_eq!(TPSS{shares: gez_zero(), all_shares: gez_zero(), total_acb: None, ..TPSS::default()}.x(), next_pre);

        // Case:
        // set_latest_post_status("(R)")
        statuses.set_latest_post_status(&default_r_af, TPSS{shares: gez!(1), all_shares: gez!(1), total_acb: None, ..TPSS::default()}.x());
        let latest_post = statuses.get_latest_post_status();
        assert_eq!(TPSS{shares: gez!(1), all_shares: gez!(1), total_acb: None, ..TPSS::default()}.x(), latest_post);
    }

    #[test]
    #[should_panic]
    fn test_panic_some_acb_registered() {
        let default_r_af = Affiliate::default_registered();

        let mut statuses = AffiliatePortfolioSecurityStatuses::new(default_sec(), None);
        statuses.set_latest_post_status(&default_r_af, TPSS{shares: gez_zero(), all_shares: gez_zero(), total_acb: Some(gez_zero()), ..TPSS::d()}.x());
    }

    #[test]
    #[should_panic]
    fn test_panic_none_acb_non_registered() {
        let default_af = Affiliate::default();

        let mut statuses = AffiliatePortfolioSecurityStatuses::new(default_sec(), None);
        statuses.set_latest_post_status(&default_af, TPSS{shares: gez_zero(), all_shares: gez_zero(), total_acb: None, ..TPSS::d()}.x());
    }
}