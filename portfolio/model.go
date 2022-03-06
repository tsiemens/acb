package portfolio

import (
	"sort"

	"github.com/tsiemens/acb/date"
)

type Currency string

const (
	DEFAULT_CURRENCY Currency = ""
	CAD              Currency = "CAD"
	USD              Currency = "USD"
)

type TxAction int

const (
	NO_ACTION TxAction = iota
	BUY
	SELL
	ROC // Return of capital
)

func (a TxAction) String() string {
	var str string = "invalid"
	switch a {
	case BUY:
		str = "Buy"
	case SELL:
		str = "Sell"
	case ROC:
		str = "RoC"
	default:
	}
	return str
}

type PortfolioSecurityStatus struct {
	Security     string
	ShareBalance uint32
	TotalAcb     float64
}

func NewEmptyPortfolioSecurityStatus(security string) *PortfolioSecurityStatus {
	return &PortfolioSecurityStatus{Security: security, ShareBalance: 0, TotalAcb: 0.0}
}

func (s *PortfolioSecurityStatus) PerShareAcb() float64 {
	if s.ShareBalance == 0 {
		return 0
	}
	return s.TotalAcb / float64(s.ShareBalance)
}

type Tx struct {
	Security                          string
	Date                              date.Date
	Action                            TxAction
	Shares                            uint32
	AmountPerShare                    float64
	Commission                        float64
	TxCurrency                        Currency
	TxCurrToLocalExchangeRate         float64
	CommissionCurrency                Currency
	CommissionCurrToLocalExchangeRate float64
	Memo                              string
	// The absolute order in which the Tx was read from file or entered.
	// Used as a tiebreak in sorting.
	ReadIndex uint32
}

type TxDelta struct {
	Tx              *Tx
	PreStatus       *PortfolioSecurityStatus
	PostStatus      *PortfolioSecurityStatus
	CapitalGain     float64
	SuperficialLoss float64
}

func (d *TxDelta) AcbDelta() float64 {
	if d.PreStatus == nil {
		return d.PostStatus.TotalAcb
	}
	return d.PostStatus.TotalAcb - d.PreStatus.TotalAcb
}

func (d *TxDelta) SuperficialLossPercent() float64 {
	return d.SuperficialLoss / (d.SuperficialLoss + d.CapitalGain)
}

type txSorter struct {
	Txs []*Tx
	// Settings
	LegacySortBuysBeforeSells bool
}

func (s *txSorter) Len() int {
	return len(s.Txs)
}

func (s *txSorter) Swap(i, j int) {
	s.Txs[i], s.Txs[j] = s.Txs[j], s.Txs[i]
}

func (s *txSorter) Less(i, j int) bool {
	iDate := s.Txs[i].Date
	jDate := s.Txs[j].Date
	if iDate.Before(jDate) {
		return true
	} else if iDate.After(jDate) {
		return false
	}

	if s.LegacySortBuysBeforeSells {
		// Tie break on order type. Buys always first, so we don't go negative.
		actionSortVal := func(action TxAction) int {
			switch action {
			case BUY:
				return 0
			case ROC:
				return 1
			case SELL:
				return 2
			default:
				return -1
			}
		}
		return actionSortVal(s.Txs[i].Action) < actionSortVal(s.Txs[j].Action)
	} else {
		// Tie break by the order read from file.
		return s.Txs[i].ReadIndex < s.Txs[j].ReadIndex
	}
}

func SortTxs(txs []*Tx, legacySortBuysBeforeSells bool) []*Tx {
	sorter := txSorter{
		Txs:                       txs,
		LegacySortBuysBeforeSells: legacySortBuysBeforeSells,
	}
	sort.Sort(&sorter)
	return sorter.Txs
}
