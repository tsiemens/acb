package portfolio

import (
	"math"
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
	capGainsYearTotals := util.NewDefaultMap[int, float64](func(_ int) float64 { return 0.0 })

	for _, d := range deltas {
		if !math.IsNaN(d.CapitalGain) {
			capGainsTotal += d.CapitalGain
			yearTotalSoFar := capGainsYearTotals.Get(d.Tx.SettlementDate.Year())
			capGainsYearTotals.Set(d.Tx.SettlementDate.Year(), yearTotalSoFar+d.CapitalGain)
		}
	}

	return &CumulativeCapitalGains{capGainsTotal, capGainsYearTotals.EjectMap()}
}

func CalcCumulativeCapitalGains(secGains map[string]*CumulativeCapitalGains) *CumulativeCapitalGains {
	var capGainsTotal float64 = 0.0
	capGainsYearTotals := util.NewDefaultMap[int, float64](func(_ int) float64 { return 0.0 })

	for _, gains := range secGains {
		capGainsTotal += gains.CapitalGainsTotal
		for year, yearGains := range gains.CapitalGainsYearTotals {
			yearTotalSoFar := capGainsYearTotals.Get(year)
			capGainsYearTotals.Set(year, yearTotalSoFar+yearGains)
		}
	}

	return &CumulativeCapitalGains{capGainsTotal, capGainsYearTotals.EjectMap()}
}
