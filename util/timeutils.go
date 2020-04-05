package util

import (
	"fmt"
	"time"
)

func DateStr(date time.Time) string {
	year, month, day := date.Date()
	return fmt.Sprintf("%d-%02d-%02d", year, month, day)
}
