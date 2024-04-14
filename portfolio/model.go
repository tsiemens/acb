package portfolio

import (
	"fmt"
	"regexp"
	"sort"
	"strings"

	"github.com/shopspring/decimal"

	"github.com/tsiemens/acb/date"
	decimal_opt "github.com/tsiemens/acb/decimal_value"
	"github.com/tsiemens/acb/util"
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
	ROC  // Return of capital
	SFLA // Superficial loss ACB adjustment
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
	case SFLA:
		str = "SfLA"
	default:
	}
	return str
}

type Affiliate struct {
	id         string
	name       string
	registered bool
}

func (a *Affiliate) Id() string {
	return a.id
}

func (a *Affiliate) Name() string {
	return a.name
}

func (a *Affiliate) Registered() bool {
	return a.registered
}

func (a *Affiliate) Default() bool {
	return strings.HasPrefix(a.Id(), "default")
}

// Technically redundant, but used for cmp, since attrs are unexported
func (a *Affiliate) Equal(other *Affiliate) bool {
	return a == other
}

func (a *Affiliate) String() string {
	return fmt.Sprintf("%v", *a)
}

var (
	registeredRe = regexp.MustCompile(`\([rR]\)`)
	extraSpaceRe = regexp.MustCompile(`  +`)
)

func NewUndedupedAffiliate(name string) Affiliate {
	// Extract registered marker
	registered := registeredRe.MatchString(name)
	prettyName := name
	if registered {
		prettyName = registeredRe.ReplaceAllString(prettyName, " ")
	}
	prettyName = extraSpaceRe.ReplaceAllString(prettyName, " ")
	prettyName = strings.TrimSpace(prettyName)
	if prettyName == "" {
		prettyName = "Default"
	}
	id := strings.ToLower(prettyName)
	if registered {
		id += " (R)"
		prettyName += " (R)"
	}

	return Affiliate{id, prettyName, registered}
}

type AffiliateDedupTable struct {
	affiliates map[string]*Affiliate
}

func NewAffiliateDedupTable() *AffiliateDedupTable {
	dt := &AffiliateDedupTable{map[string]*Affiliate{}}
	// Insert the default affiliates (just to ensure they get a consistent
	// capitalization)
	dt.DedupedAffiliate("Default")
	dt.DedupedAffiliate("Default (R)")
	return dt
}

// Used by io.go while loading Txs
var GlobalAffiliateDedupTable = NewAffiliateDedupTable()

func (t *AffiliateDedupTable) DedupedAffiliate(name string) *Affiliate {
	preDedupedAffiliate := NewUndedupedAffiliate(name)
	if affiliate, ok := t.affiliates[preDedupedAffiliate.Id()]; ok {
		return affiliate
	}

	// Add to the dedup table
	affiliate := &Affiliate{}
	*affiliate = preDedupedAffiliate
	t.affiliates[affiliate.Id()] = affiliate
	return affiliate
}

func (t *AffiliateDedupTable) MustGet(id string) *Affiliate {
	af, ok := t.affiliates[id]
	util.Assertf(ok, "AffiliateDedupTable could not find Affiliate \"%s\"", id)
	return af
}

func (t *AffiliateDedupTable) GetDefaultAffiliate() *Affiliate {
	return t.MustGet("default")
}

type PortfolioSecurityStatus struct {
	Security                  string
	ShareBalance              decimal.Decimal
	AllAffiliatesShareBalance decimal.Decimal
	TotalAcb                  decimal_opt.DecimalOpt
}

func NewEmptyPortfolioSecurityStatus(security string) *PortfolioSecurityStatus {
	return &PortfolioSecurityStatus{Security: security}
}

func (s *PortfolioSecurityStatus) PerShareAcb() decimal_opt.DecimalOpt {
	if s.ShareBalance.IsZero() {
		return decimal_opt.Zero
	}
	return s.TotalAcb.DivD(s.ShareBalance)
}

type SFLInput struct {
	SuperficialLoss decimal_opt.DecimalOpt
	Force           bool
}

func (i SFLInput) Equal(other SFLInput) bool {
	return i.SuperficialLoss.Equal(other.SuperficialLoss) && i.Force == other.Force
}

func (i SFLInput) String() string {
	return fmt.Sprintf("%v%s", i.SuperficialLoss, util.Tern(i.Force, " (forced)", ""))
}

// We want to be able to call .Equal on this value, but it doesn't quite work
// correctly with the raw Optional (cmp package doesn't seem to work that well with
// generics).
type SFLInputOpt struct {
	util.Optional[SFLInput]
}

func NewSFLInputOpt(v SFLInput) SFLInputOpt {
	return SFLInputOpt{util.NewOptional(v)}
}

func (b SFLInputOpt) Equal(other SFLInputOpt) bool {
	needEqualityCheck, equal := b.Optional.NeedValueEqualityCheck(other.Optional)
	if needEqualityCheck {
		return b.Optional.MustGet().Equal(other.Optional.MustGet())
	}
	return equal
}

func (b SFLInputOpt) String() string {
	return b.Optional.String()
}

// TODO the exchange rates here should perhaps be more explicitly optional, but
// DecimalOpt defaults to zero, rather than unset. We'd want to use Optional, which
// is less convenient to use. Zero isn't a valid rate ever so it's ok for now.

type Tx struct {
	Security                          string
	TradeDate                         date.Date
	SettlementDate                    date.Date
	Action                            TxAction
	Shares                            decimal.Decimal
	AmountPerShare                    decimal.Decimal
	Commission                        decimal.Decimal
	TxCurrency                        Currency
	TxCurrToLocalExchangeRate         decimal.Decimal
	CommissionCurrency                Currency
	CommissionCurrToLocalExchangeRate decimal.Decimal
	Memo                              string
	Affiliate                         *Affiliate

	// More commonly optional fields/columns

	// The total superficial loss for the transaction, as explicitly
	// specified by the user. May be cross-validated against calculated SFL to emit
	// warnings. If specified, the user is also required to specify one or more
	// SfLA Txs following this one, accounting for all shares experiencing the loss.
	// NOTE: This is always a negative (or zero) value in CAD, so that it matches the
	// displayed value
	// SpecifiedSuperficialLoss util.Optional[SFLInput]
	SpecifiedSuperficialLoss SFLInputOpt

	// The absolute order in which the Tx was read from file or entered.
	// Used as a tiebreak in sorting.
	ReadIndex uint32
}

type TxDelta struct {
	Tx          *Tx
	PreStatus   *PortfolioSecurityStatus
	PostStatus  *PortfolioSecurityStatus
	CapitalGain decimal_opt.DecimalOpt

	SuperficialLoss decimal_opt.DecimalOpt
	// A ratio, representing <N reacquired shares which suffered SFL> / <N sold shares>
	SuperficialLossRatio      util.DecimalRatio
	PotentiallyOverAppliedSfl bool
}

func (d *TxDelta) String() string {
	return fmt.Sprintf(
		"Tx: %v, PreSt: %v, PostSt: %v, Gain: %v, Sfl: %v, SflR: %v, POASfl: %v",
		d.Tx, d.PreStatus, d.PostStatus, d.CapitalGain, d.SuperficialLoss,
		d.SuperficialLossRatio, d.PotentiallyOverAppliedSfl)
}

func (d *TxDelta) AcbDelta() decimal_opt.DecimalOpt {
	if d.PreStatus == nil {
		return d.PostStatus.TotalAcb
	}
	return d.PostStatus.TotalAcb.Sub(d.PreStatus.TotalAcb)
}

func (d *TxDelta) IsSuperficialLoss() bool {
	return !d.SuperficialLoss.IsNull && !d.SuperficialLoss.IsZero()
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
	iDate := s.Txs[i].SettlementDate
	jDate := s.Txs[j].SettlementDate
	if iDate.Before(jDate) {
		return true
	} else if iDate.After(jDate) {
		return false
	}

	// Tie break by the order read from file.
	return s.Txs[i].ReadIndex < s.Txs[j].ReadIndex
}

func SortTxs(txs []*Tx) []*Tx {
	sorter := txSorter{
		Txs: txs,
	}
	sort.Sort(&sorter)
	return sorter.Txs
}
