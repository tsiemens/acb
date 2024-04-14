package fx

import (
	"fmt"

	"github.com/shopspring/decimal"
	"github.com/tsiemens/acb/date"
)

type DailyRate struct {
	Date               date.Date
	ForeignToLocalRate decimal.Decimal
}

func (r DailyRate) Equal(other DailyRate) bool {
	return r.Date.Equal(other.Date) && r.ForeignToLocalRate.Equal(other.ForeignToLocalRate)
}

func (r DailyRate) String() string {
	return fmt.Sprintf("%s : %s", r.Date.String(), r.ForeignToLocalRate)
}
