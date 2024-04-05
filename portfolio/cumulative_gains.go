package portfolio

import (
	"sort"

	_ "github.com/shopspring/decimal"
	decimal "github.com/tsiemens/acb/decimal_value"
	"github.com/tsiemens/acb/util"
)

type CumulativeCapitalGains struct {
	CapitalGainsTotal      decimal.Decimal
	CapitalGainsYearTotals map[int]decimal.Decimal
}

func (g *CumulativeCapitalGains) CapitalGainsYearTotalsKeysSorted() []int {
	years := util.IntDecimalMapKeys(g.CapitalGainsYearTotals)
	sort.Ints(years)
	return years
}

func CalcSecurityCumulativeCapitalGains(deltas []*TxDelta) *CumulativeCapitalGains {
	var capGainsTotal decimal.Decimal
	capGainsYearTotals := util.NewDefaultMap[int, decimal.Decimal](func(_ int) decimal.Decimal { return decimal.Zero })

	for _, d := range deltas {
		if !d.CapitalGain.IsNull {
			capGainsTotal = capGainsTotal.Add(d.CapitalGain)
			yearTotalSoFar := capGainsYearTotals.Get(d.Tx.SettlementDate.Year())
			capGainsYearTotals.Set(d.Tx.SettlementDate.Year(), yearTotalSoFar.Add(d.CapitalGain))
		}
	}

	return &CumulativeCapitalGains{capGainsTotal, capGainsYearTotals.EjectMap()}
}

func CalcCumulativeCapitalGains(secGains map[string]*CumulativeCapitalGains) *CumulativeCapitalGains {
	var capGainsTotal decimal.Decimal
	capGainsYearTotals := util.NewDefaultMap[int, decimal.Decimal](func(_ int) decimal.Decimal { return decimal.Zero })

	for _, gains := range secGains {
		capGainsTotal = capGainsTotal.Add(gains.CapitalGainsTotal)
		for year, yearGains := range gains.CapitalGainsYearTotals {
			yearTotalSoFar := capGainsYearTotals.Get(year)
			capGainsYearTotals.Set(year, yearTotalSoFar.Add(yearGains))
		}
	}

	return &CumulativeCapitalGains{capGainsTotal, capGainsYearTotals.EjectMap()}
}
