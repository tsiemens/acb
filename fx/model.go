package fx

import (
	"fmt"

	"github.com/tsiemens/acb/date"
)

type DailyRate struct {
	Date               date.Date
	ForeignToLocalRate float64
}

func (r *DailyRate) String() string {
	return fmt.Sprintf("%s : %f", r.Date.String(), r.ForeignToLocalRate)
}
