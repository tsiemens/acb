package portfolio

import (
	"sort"

	"github.com/tsiemens/acb/util"
)

type CumulativeCapitalGains struct {
	CapitalGainsTotal      float64
	CapitalGainsYearTotals map[int]float64
}

func (g *CumulativeCapitalGains) CapitalGainsYearTotalsKeysSorted() []int {
	years := util.IntFloat64MapKeys(g.CapitalGainsYearTotals)
	sort.Ints(years)
	return years
}

func CalcSecurityCumulativeCapitalGains(deltas []*TxDelta) *CumulativeCapitalGains {
	var capGainsTotal float64 = 0.0
	capGainsYearTotals := map[int]float64{}

	for _, d := range deltas {
		capGainsTotal += d.CapitalGain
		yearTotalSoFar, ok := capGainsYearTotals[d.Tx.SettlementDate.Year()]
		if !ok {
			yearTotalSoFar = 0.0
		}
		capGainsYearTotals[d.Tx.SettlementDate.Year()] = yearTotalSoFar + d.CapitalGain
	}

	return &CumulativeCapitalGains{capGainsTotal, capGainsYearTotals}
}

func CalcCumulativeCapitalGains(secGains map[string]*CumulativeCapitalGains) *CumulativeCapitalGains {
	var capGainsTotal float64 = 0.0
	capGainsYearTotals := map[int]float64{}

	for _, gains := range secGains {
		capGainsTotal += gains.CapitalGainsTotal
		for year, yearGains := range gains.CapitalGainsYearTotals {
			yearTotalSoFar, ok := capGainsYearTotals[year]
			if !ok {
				yearTotalSoFar = 0.0
			}
			capGainsYearTotals[year] = yearTotalSoFar + yearGains
		}
	}

	return &CumulativeCapitalGains{capGainsTotal, capGainsYearTotals}
}
