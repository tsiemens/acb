package decimal_value

import (
	"github.com/shopspring/decimal"
)

var Zero = Decimal{Decimal: decimal.Zero}
var Null = Decimal{IsNull: true}

type Decimal struct {
	decimal.Decimal
	IsNull bool
}

func New(value decimal.Decimal) Decimal {
	return Decimal{Decimal: value}
}

func NewFromInt(value int64) Decimal {
	return Decimal{Decimal: decimal.NewFromInt(value)}
}

func NewFromFloat(value float64) Decimal {
	return Decimal{Decimal: decimal.NewFromFloat(value)}
}

func NewFromFloatWithExponent(value float64, exponent int32) Decimal {
	return Decimal{Decimal: decimal.NewFromFloatWithExponent(value, exponent)}
}

func NewFromString(value string) (Decimal, error) {
	d, err := decimal.NewFromString(value)
	if err != nil {
		return Null, err
	}

	return Decimal{Decimal: d}, nil
}

func RequireFromString(value string) Decimal {
	return Decimal{Decimal: decimal.RequireFromString(value)}
}

func Abs(d Decimal) Decimal {
	return Decimal{Decimal: d.Decimal.Abs()}
}

func Min(first Decimal, rest ...Decimal) Decimal {
	restValues := make([]decimal.Decimal, len(rest))

	for i, d := range rest {
		restValues[i] = d.Decimal
	}

	return Decimal{Decimal: decimal.Min(first.Decimal, restValues...)}
}

func (d Decimal) Add(d2 Decimal) Decimal {
	if d.IsNull || d2.IsNull {
		return Null
	}
	return Decimal{Decimal: d.Decimal.Add(d2.Decimal)}
}

func (d Decimal) Neg() Decimal {
	if d.IsNull {
		return Null
	}
	return Decimal{Decimal: d.Decimal.Neg()}
}

func (d Decimal) Sub(d2 Decimal) Decimal {
	if d.IsNull || d2.IsNull {
		return Null
	}
	return Decimal{Decimal: d.Decimal.Sub(d2.Decimal)}
}

func (d Decimal) Mul(d2 Decimal) Decimal {
	if d.IsNull || d2.IsNull {
		return Null
	}
	return Decimal{Decimal: d.Decimal.Mul(d2.Decimal)}
}

func (d Decimal) Div(d2 Decimal) Decimal {
	if d.IsNull || d2.IsNull {
		return Null
	}
	return Decimal{Decimal: d.Decimal.Div(d2.Decimal)}
}

func (d Decimal) Equal(d2 Decimal) bool {
	if d.IsNull == d2.IsNull {
		return true
	}

	if d.IsNull != d2.IsNull {
		return false
	}

	return d.Decimal.Equal(d2.Decimal)
}

func (d Decimal) GreaterThan(d2 Decimal) bool {
	if d.IsNull || d2.IsNull {
		return false
	}

	return d.Decimal.GreaterThan(d2.Decimal)
}

func (d Decimal) GreaterThanOrEqual(d2 Decimal) bool {
	if d.IsNull || d2.IsNull {
		return false
	}

	return d.Decimal.GreaterThanOrEqual(d2.Decimal)
}

func (d Decimal) LessThan(d2 Decimal) bool {
	if d.IsNull || d2.IsNull {
		return false
	}

	return d.Decimal.LessThan(d2.Decimal)
}

func (d Decimal) LessThanOrEqual(d2 Decimal) bool {
	if d.IsNull || d2.IsNull {
		return false
	}

	return d.Decimal.LessThanOrEqual(d2.Decimal)
}

func (d Decimal) IsZero() bool {
	if d.IsNull {
		return true
	}

	return d.Decimal.IsZero()
}

func (d Decimal) IsPositive() bool {
	if d.IsNull {
		return false
	}

	return d.Decimal.IsPositive()
}

func (d Decimal) IsNegative() bool {
	if d.IsNull {
		return false
	}

	return d.Decimal.IsNegative()
}

func (d Decimal) String() string {
	if d.IsNull {
		return "NaN"
	}

	return d.Decimal.String()
}
