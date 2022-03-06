package test

import (
	"testing"
	"time"

	"github.com/stretchr/testify/require"

	"github.com/tsiemens/acb/date"
)

func TestDate(t *testing.T) {
	rq := require.New(t)

	d1 := date.New(2022, 1, 2)
	d2, err := date.Parse(date.DefaultFormat, "2022-01-02")
	rq.Nil(err)
	rq.Equal(d1, d2)
	rq.Equal("2022-01-02", d1.String())

	d2, err = date.Parse(date.DefaultFormat, "2022-01-02 xxxx")
	rq.NotNil(err)

	d3 := d1.AddDays(2)
	rq.Equal("2022-01-04", d3.String())

	defaultDate := date.Date{}
	rq.Equal(defaultDate, date.New(1, time.January, 1))
}
