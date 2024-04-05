package fx

import (
	"fmt"

	"github.com/tsiemens/acb/date"
	decimal "github.com/tsiemens/acb/decimal_value"
)

type DailyRate struct {
	Date               date.Date
	ForeignToLocalRate decimal.Decimal
}

func (r *DailyRate) String() string {
	return fmt.Sprintf("%s : %s", r.Date.String(), r.ForeignToLocalRate)
}
