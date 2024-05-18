use super::{superficial_loss::get_superficial_loss_ratio, AffiliatePortfolioSecurityStatuses};

pub fn txs_to_delta_list() {
    // TODO this is a placeholder for now

    // Reference our protected fns so they aren't
    // marked with warnings for unuse.
    let r = get_superficial_loss_ratio(
        0, &Vec::new(),
        &AffiliatePortfolioSecurityStatuses::new("foo".to_string(), None)
    ).unwrap().unwrap();

    println!("{:#?}, {:#?}, {:#?}",
        r.sfl_ratio, r.acb_adjust_affiliate_ratios, r.fewer_remaining_shares_than_sfl_shares);
}