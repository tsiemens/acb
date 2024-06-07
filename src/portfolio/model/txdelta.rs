use std::rc::Rc;

use rust_decimal::Decimal;

use crate::util::{
    decimal::{GreaterEqualZeroDecimal, NegDecimal, PosDecimal},
    math::PosDecimalRatio,
};

use super::tx::Tx;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct PortfolioSecurityStatus {
    pub security: String,
    pub share_balance: GreaterEqualZeroDecimal,
    pub all_affiliate_share_balance: GreaterEqualZeroDecimal,
    pub total_acb: Option<GreaterEqualZeroDecimal>, // None for registered affiliates
}

impl PortfolioSecurityStatus {
    pub fn per_share_acb(&self) -> Option<GreaterEqualZeroDecimal> {
        if self.total_acb.is_none() {
            return None;
        }
        Some(match PosDecimal::try_from(*self.share_balance) {
            Ok(sb) => self.total_acb.unwrap().div(sb),
            Err(_) => GreaterEqualZeroDecimal::zero(), // balance of zero
        })
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct DeltaSflInfo {
    // In CAD
    pub superficial_loss: NegDecimal,
    // A ratio, representing <N reacquired shares which suffered SFL> / <N sold shares>
    pub ratio: PosDecimalRatio,
    pub potentially_over_applied: bool,
}

#[derive(PartialEq, Eq, Clone, Debug)]

pub struct TxDelta {
    pub tx: Tx,
    pub pre_status: Rc<PortfolioSecurityStatus>,
    pub post_status: Rc<PortfolioSecurityStatus>,
    pub capital_gain: Option<Decimal>,
    pub sfl: Option<DeltaSflInfo>,
}

impl TxDelta {
    pub fn acb_delta(&self) -> Option<Decimal> {
        if self.pre_status.total_acb.is_none()
            || self.post_status.total_acb.is_none()
        {
            return None;
        }
        Some(
            *self.post_status.total_acb.unwrap()
                - *self.pre_status.total_acb.unwrap(),
        )
    }

    pub fn is_superficial_loss(&self) -> bool {
        match &self.sfl {
            Some(sfl) => !sfl.superficial_loss.is_zero(),
            None => false,
        }
    }
}
