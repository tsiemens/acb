package fx

import (
	"fmt"
	"time"
)

type DailyRate struct {
	Date               time.Time
	ForeignToLocalRate float64
}

func (r DailyRate) String() string {
	year, month, day := r.Date.Date()
	return fmt.Sprintf("%d-%02d-%02d : %f", year, month, day, r.ForeignToLocalRate)
}
