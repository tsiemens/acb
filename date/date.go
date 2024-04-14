package date

import (
	"fmt"
	"time"

	"github.com/tsiemens/acb/util"
)

const DefaultFormat = "2006-01-02"

// Represents a pure date, with no effects from time zones, or time.
// Represented in UTC time at 00:00:00
type Date struct {
	time time.Time
}

func (d Date) UTCTime() time.Time {
	return d.time
}

func New(year uint32, month time.Month, day uint32) Date {
	return Date{time.Date(int(year), month, int(day), 0, 0, 0, 0, time.UTC)}
}

func NewFromTime(t time.Time) Date {
	return New(uint32(t.Year()), t.Month(), uint32(t.Day()))
}

func (d Date) isPureUtcDate() bool {
	other := NewFromTime(d.time)
	return d == other
}

func (d Date) Equal(other Date) bool {
	return d.time.Equal(other.time)
}

func Parse(dFmt string, dateStr string) (Date, error) {
	tm, err := time.Parse(dFmt, dateStr)
	if err != nil {
		return Date{}, err
	}
	d := Date{tm}
	if !d.isPureUtcDate() {
		return Date{}, fmt.Errorf("Format %v and string %v did not produce a pure date", dFmt, dateStr)
	}
	return d, nil
}

var TodaysDateForTest Date = Date{}

func Today() Date {
	if TodaysDateForTest != (Date{}) {
		return TodaysDateForTest
	}
	return NewFromTime(time.Now())
}

// After reports whether the date instant d is after u.
func (d Date) After(u Date) bool {
	return d.time.After(u.time)
}

// Before reports whether the date instant d is before u.
func (d Date) Before(u Date) bool {
	return d.time.Before(u.time)
}

func (d Date) String() string {
	year, month, day := d.time.Date()
	return fmt.Sprintf("%d-%02d-%02d", year, month, day)
}

func (d Date) AddDays(nDays int) Date {
	newDate := Date{d.time.AddDate(0, 0, nDays)}
	util.Assert(newDate.isPureUtcDate(), "time.Time.Add of days resulted in time-of-day change")
	return newDate
}

func (d Date) Parts() (int, time.Month, int) {
	return d.time.Date()
}

func (d Date) Year() int {
	return d.time.Year()
}
