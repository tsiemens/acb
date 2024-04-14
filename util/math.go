package util

import (
	"github.com/shopspring/decimal"
)

type DecimalRatio struct {
	Numerator   decimal.Decimal
	Denominator decimal.Decimal
}

func (r *DecimalRatio) Valid() bool {
	return !r.Denominator.IsZero()
}

func (r *DecimalRatio) ToDecimal() decimal.Decimal {
	return r.Numerator.Div(r.Denominator)
}
