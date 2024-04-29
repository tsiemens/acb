package portfolio

import (
	"sort"

	decimal_opt "github.com/tsiemens/acb/decimal_value"
	"github.com/tsiemens/acb/util"
)

type CumulativeCapitalGains struct {
	CapitalGainsTotal      decimal_opt.DecimalOpt
	CapitalGainsYearTotals map[int]decimal_opt.DecimalOpt
}

func (g *CumulativeCapitalGains) CapitalGainsYearTotalsKeysSorted() []int {
	years := util.IntDecimalOptMapKeys(g.CapitalGainsYearTotals)
	sort.Ints(years)
	return years
}

func CalcSecurityCumulativeCapitalGains(deltas []*TxDelta) *CumulativeCapitalGains {
	var capGainsTotal decimal_opt.DecimalOpt
	capGainsYearTotals := util.NewDefaultMap(
		func(_ int) decimal_opt.DecimalOpt { return decimal_opt.Zero })

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
	var capGainsTotal decimal_opt.DecimalOpt
	capGainsYearTotals := util.NewDefaultMap(
		func(_ int) decimal_opt.DecimalOpt { return decimal_opt.Zero })

	for _, gains := range secGains {
		capGainsTotal = capGainsTotal.Add(gains.CapitalGainsTotal)
		for year, yearGains := range gains.CapitalGainsYearTotals {
			yearTotalSoFar := capGainsYearTotals.Get(year)
			capGainsYearTotals.Set(year, yearTotalSoFar.Add(yearGains))
		}
	}

	return &CumulativeCapitalGains{capGainsTotal, capGainsYearTotals.EjectMap()}
}
