package portfolio

import (
	"time"

	"github.com/tsiemens/acb/util"
)

// Security	Date	Transaction	Amount	Shares	Amount/Share	Commission	Capital Gain (Loss)	Share Balance	Change in ACB	New ACB	New ACB/Share	Memo	Foreign Currency Transaction	Exchange Rate	Amount in Foreign Currency	Commission in Foreign Corrency	T-Slip Capital Gain

type Currency string

const (
	CAD Currency = "CAD"
	USD Currency = "USD"
)

type TxAction int

const (
	BUY TxAction = iota
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
		util.Assert(false, "Invalid action", a)
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
	Date                              time.Time
	Action                            TxAction
	Shares                            uint32
	PricePerShare                     float64
	Commission                        float64
	TxCurrency                        Currency
	TxCurrToLocalExchangeRate         float64
	CommissionCurrency                Currency
	CommissionCurrToLocalExchangeRate float64
	Memo                              string
}

type TxDelta struct {
	Tx          *Tx
	PreStatus   *PortfolioSecurityStatus
	PostStatus  *PortfolioSecurityStatus
	CapitalGain float64
}

func (d *TxDelta) AcbDelta() float64 {
	if d.PreStatus == nil {
		return d.PostStatus.TotalAcb
	}
	return d.PostStatus.TotalAcb - d.PreStatus.TotalAcb
}

type txSorter struct {
	Txs []*Tx
}

func (s *txSorter) Len() int {
	return len(s.Txs)
}

func (s *txSorter) Swap(i, j int) {
	s.Txs[i], s.Txs[j] = s.Txs[j], s.Txs[i]
}

func (s *txSorter) Less(i, j int) bool {
	return s.Txs[i].Date.Unix() < s.Txs[j].Date.Unix()
}
