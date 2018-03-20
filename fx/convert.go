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

type CurrencyConverter interface {
	ConvertToLocal(value float32, t time.Time) (float32, error)
	ConvertToForeign(value float32, t time.Time) (float32, error)
}

type CadUsdConverter struct {
}

func (c *CadUsdConverter) ConvertToLocal(value float32, t time.Time) (float32, error) {
	return 0.0, nil
}

func (c *CadUsdConverter) ConvertToForeign(value float32, t time.Time) (float32, error) {
	return 0.0, nil
}
