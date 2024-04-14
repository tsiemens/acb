package decimal_value

import (
	"github.com/shopspring/decimal"
)

var Zero = DecimalOpt{Decimal: decimal.Zero}
var Null = DecimalOpt{IsNull: true}

type DecimalOpt struct {
	Decimal decimal.Decimal
	IsNull  bool
}

func New(value decimal.Decimal) DecimalOpt {
	return DecimalOpt{Decimal: value}
}

func NewFromInt(value int64) DecimalOpt {
	return DecimalOpt{Decimal: decimal.NewFromInt(value)}
}

func NewFromFloat(value float64) DecimalOpt {
	return DecimalOpt{Decimal: decimal.NewFromFloat(value)}
}

func NewFromFloatWithExponent(value float64, exponent int32) DecimalOpt {
	return DecimalOpt{Decimal: decimal.NewFromFloatWithExponent(value, exponent)}
}

func NewFromString(value string) (DecimalOpt, error) {
	d, err := decimal.NewFromString(value)
	if err != nil {
		return Null, err
	}

	return DecimalOpt{Decimal: d}, nil
}

func RequireFromString(value string) DecimalOpt {
	return DecimalOpt{Decimal: decimal.RequireFromString(value)}
}

func Abs(d DecimalOpt) DecimalOpt {
	if d.IsNull {
		return Null
	}

	return DecimalOpt{Decimal: d.Decimal.Abs()}
}

func (d DecimalOpt) Add(d2 DecimalOpt) DecimalOpt {
	if d.IsNull || d2.IsNull {
		return Null
	}
	return DecimalOpt{Decimal: d.Decimal.Add(d2.Decimal)}
}

func (d DecimalOpt) AddD(d2 decimal.Decimal) DecimalOpt {
	return d.Add(New(d2))
}

func (d DecimalOpt) Neg() DecimalOpt {
	if d.IsNull {
		return Null
	}
	return DecimalOpt{Decimal: d.Decimal.Neg()}
}

func (d DecimalOpt) Sub(d2 DecimalOpt) DecimalOpt {
	if d.IsNull || d2.IsNull {
		return Null
	}
	return DecimalOpt{Decimal: d.Decimal.Sub(d2.Decimal)}
}

func (d DecimalOpt) SubD(d2 decimal.Decimal) DecimalOpt {
	return d.Sub(New(d2))
}

func (d DecimalOpt) Mul(d2 DecimalOpt) DecimalOpt {
	if d.IsNull || d2.IsNull {
		return Null
	}
	return DecimalOpt{Decimal: d.Decimal.Mul(d2.Decimal)}
}

func (d DecimalOpt) MulD(d2 decimal.Decimal) DecimalOpt {
	return d.Mul(New(d2))
}

func (d DecimalOpt) Div(d2 DecimalOpt) DecimalOpt {
	if d.IsNull || d2.IsNull {
		return Null
	}
	if d2.IsZero() {
		return Null
	}
	return DecimalOpt{Decimal: d.Decimal.Div(d2.Decimal)}
}

func (d DecimalOpt) DivD(d2 decimal.Decimal) DecimalOpt {
	return d.Div(New(d2))
}

func (d DecimalOpt) Equal(d2 DecimalOpt) bool {
	if d.IsNull == d2.IsNull {
		return true
	}

	if d.IsNull != d2.IsNull {
		return false
	}

	return d.Decimal.Equal(d2.Decimal)
}

func (d DecimalOpt) GreaterThan(d2 DecimalOpt) bool {
	if d.IsNull || d2.IsNull {
		return false
	}

	return d.Decimal.GreaterThan(d2.Decimal)
}

func (d DecimalOpt) GreaterThanOrEqual(d2 DecimalOpt) bool {
	if d.IsNull || d2.IsNull {
		return false
	}

	return d.Decimal.GreaterThanOrEqual(d2.Decimal)
}

func (d DecimalOpt) LessThan(d2 DecimalOpt) bool {
	if d.IsNull || d2.IsNull {
		return false
	}

	return d.Decimal.LessThan(d2.Decimal)
}

func (d DecimalOpt) LessThanOrEqual(d2 DecimalOpt) bool {
	if d.IsNull || d2.IsNull {
		return false
	}

	return d.Decimal.LessThanOrEqual(d2.Decimal)
}

func (d DecimalOpt) IsZero() bool {
	if d.IsNull {
		return false
	}

	return d.Decimal.IsZero()
}

func (d DecimalOpt) IsPositive() bool {
	if d.IsNull {
		return false
	}

	return d.Decimal.IsPositive()
}

func (d DecimalOpt) IsNegative() bool {
	if d.IsNull {
		return false
	}

	return d.Decimal.IsNegative()
}

func (d DecimalOpt) String() string {
	if d.IsNull {
		return "NaN"
	}

	return d.Decimal.String()
}

func (d DecimalOpt) StringFixed(places int32) string {
	if d.IsNull {
		return "NaN"
	}

	return d.Decimal.StringFixed(places)
}
