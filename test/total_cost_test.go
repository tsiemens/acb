package test

import (
	"fmt"
	"sort"
	"strings"
	"testing"

	"github.com/shopspring/decimal"
	"github.com/stretchr/testify/require"
	"github.com/tsiemens/acb/app/outfmt"
	"github.com/tsiemens/acb/date"
	"github.com/tsiemens/acb/decimal_value"
	ptf "github.com/tsiemens/acb/portfolio"
)

type td struct {
	Sec      string
	Settle   string
	TotalAcb string
}

func getDelta(td *td) (*ptf.TxDelta, error) {
	settle, err := date.Parse(date.DefaultFormat, td.Settle)
	if err != nil {
		return nil, fmt.Errorf("date %v: %w", td.Settle, err)
	}

	acb := decimal_value.Null
	if td.TotalAcb != "" {
		var err error
		acb, err = decimal_value.NewFromString(td.TotalAcb)
		if err != nil {
			return nil, fmt.Errorf("TotalAcb %v: %w", td.TotalAcb, err)
		}
	}

	return &ptf.TxDelta{
		Tx: &ptf.Tx{
			Security:       td.Sec,
			SettlementDate: settle,
		},
		PostStatus: &ptf.PortfolioSecurityStatus{
			Security: td.Sec,
			TotalAcb: acb,
		},
	}, nil
}

func getDeltas(v []*td) (ret []*ptf.TxDelta, _ error) {
	for _, i := range v {
		d, err := getDelta(i)
		if err != nil {
			return nil, fmt.Errorf("getDelta %v: %w", i, err)
		}
		ret = append(ret, d)
	}
	return ret, nil
}

func renderTable(rq *require.Assertions, rt *ptf.RenderTable) string {
	b := &strings.Builder{}
	wr := outfmt.NewSTDWriter(b)
	rq.NoError(wr.PrintRenderTable(outfmt.Costs, "Table of", rt))
	return b.String()
}

func fixupRows(startidx int, v [][]string) {
	for i, r := range v {
		for j := startidx; j < len(r); j++ {
			c := strings.TrimSpace(r[j])
			v[i][j] = "$" + decimal_value.RequireFromString(c).StringFixed(2)
		}
	}
}
func TestRenderTotalCosts(t *testing.T) {
	for _, tc := range []struct {
		name  string
		reorg func(data []*td)
	}{
		{
			name:  "none",
			reorg: func(data []*td) {},
		},
		{
			name: "by-security",
			reorg: func(data []*td) {
				sort.SliceStable(data, func(i, j int) bool {
					return data[i].Sec < data[j].Sec
				})
			},
		},
		{
			name: "reverse",
			reorg: func(data []*td) {
				for i, j := 0, len(data)-1; i < j; i, j = i+1, j-1 {
					data[i], data[j] = data[j], data[i]
				}
			},
		},
		{
			name: "by-acb-sec",
			reorg: func(data []*td) {
				sort.SliceStable(data, func(i, j int) bool {
					_get := func(v string) decimal.Decimal {
						if v == "" {
							return decimal.NewFromFloat(-1)
						}
						return decimal.RequireFromString(v)
					}
					ii := _get(data[i].TotalAcb)
					jj := _get(data[j].TotalAcb)
					if !ii.Equal(jj) {
						return ii.LessThan(jj)
					}
					return data[i].Sec < data[j].Sec
				})
			},
		},
	} {
		t.Run(fmt.Sprint("reorg:", tc.name), func(t *testing.T) {
			data := []*td{
				{Sec: "SECA", Settle: "2001-01-13", TotalAcb: "100"},
				{Sec: "XXXX", Settle: "2001-02-14", TotalAcb: "90"},
				{Sec: "SECA", Settle: "2001-03-15", TotalAcb: "0"},
				{Sec: "XXXX", Settle: "2001-04-16", TotalAcb: "80"},
				{Sec: "SECA", Settle: "2001-05-17", TotalAcb: "200"},
				{Sec: "XXXX", Settle: "2001-05-17", TotalAcb: "70"},
				{Sec: "SECA", Settle: "2003-01-01", TotalAcb: "0"},
				{Sec: "SECA", Settle: "2003-01-02", TotalAcb: "150"},
				{Sec: "XXXX", Settle: "2003-01-02", TotalAcb: "35"},
				{Sec: "TFSA", Settle: "2003-01-02", TotalAcb: ""},
				{Sec: "SECA", Settle: "2003-01-03", TotalAcb: "0"},
			}
			tc.reorg(data)

			for _, d := range data {
				t.Log(d)
			}

			rq := require.New(t)
			allDeltas, err := getDeltas(data)
			rq.NoError(err)
			costs := ptf.RenderTotalCosts(allDeltas, false)

			notes := []string{"2003-01-02 (TFSA) ignored transaction from registered affiliate"}

			exp := &ptf.RenderTable{
				Header: []string{"Date", "Total", "SECA", "XXXX"},
				Rows: [][]string{
					{"2001-01-13", "100", "100", " 0"},
					{"2001-02-14", "190", "100", "90"},
					{"2001-03-15", " 90", "  0", "90"},
					{"2001-04-16", " 80", "  0", "80"},
					{"2001-05-17", "270", "200", "70"},
					{"2003-01-01", " 70", "  0", "70"},
					{"2003-01-02", "185", "150", "35"},
					{"2003-01-03", " 35", "  0", "35"},
				},
				Notes: notes,
			}
			fixupRows(1, exp.Rows)

			actual := renderTable(rq, costs.Total)
			t.Log("Actual:\n" + actual)
			rq.Equal(renderTable(rq, exp), actual)

			expYear := &ptf.RenderTable{
				Header: []string{"Year", "Date", "Total", "SECA", "XXXX"},
				Rows: [][]string{
					{"2001", "2001-05-17", "270", "200", "70"},
					{"2003", "2003-01-02", "185", "150", "35"},
				},
				Notes: notes,
			}
			fixupRows(2, expYear.Rows)
			yearActual := renderTable(rq, costs.Yearly)
			t.Log("\n" + yearActual)
			rq.Equal(renderTable(rq, expYear), yearActual)
		})
	}
}
