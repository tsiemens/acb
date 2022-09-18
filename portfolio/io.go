package portfolio

import (
	"bytes"
	"encoding/csv"
	"fmt"
	"io"
	"strconv"
	"strings"

	"github.com/tsiemens/acb/date"
	"github.com/tsiemens/acb/fx"
	"github.com/tsiemens/acb/util"
)

const (
	CsvDateFormatDefault string = "2006-01-02"
)

var CsvDateFormat string = CsvDateFormatDefault

type ColParser func(string, *Tx) error

var colParserMap = map[string]ColParser{
	"security":                 parseSecurity,
	"trade date":               parseTradeDate,
	"date":                     parseSettlementDate,
	"settlement date":          parseSettlementDate,
	"action":                   parseAction,
	"shares":                   parseShares,
	"amount/share":             parseAmountPerShare,
	"commission":               parseCommission,
	"currency":                 parseTxCurr,
	"exchange rate":            parseTxFx,
	"commission currency":      parseCommissionCurr,
	"commission exchange rate": parseCommissionFx,
	"superficial loss":         parseSuperficialLoss,
	"affiliate":                parseAffiliate,
	"memo":                     parseMemo,
}

var ColNames []string

func init() {
	ColNames = make([]string, 0, len(colParserMap))
	for name, _ := range colParserMap {
		ColNames = append(ColNames, name)
	}
}

func DefaultTx() *Tx {
	return &Tx{
		Security: "", SettlementDate: date.Date{}, Action: NO_ACTION,
		Shares: 0, AmountPerShare: 0.0, Commission: 0.0,
		TxCurrency: DEFAULT_CURRENCY, TxCurrToLocalExchangeRate: 0.0,
		CommissionCurrency: DEFAULT_CURRENCY, CommissionCurrToLocalExchangeRate: 0.0,
		Affiliate: GlobalAffiliateDedupTable.GetDefaultAffiliate(),
	}
}

func CheckTxSanity(tx *Tx) error {
	if tx.Security == "" {
		return fmt.Errorf("Transaction has no security")
	} else if (tx.TradeDate == date.Date{}) {
		return fmt.Errorf("Transaction has no trade date")
	} else if (tx.SettlementDate == date.Date{}) {
		return fmt.Errorf("Transaction has no settlement date")
	} else if tx.Action == NO_ACTION {
		return fmt.Errorf("Transaction has no action (Buy, Sell, RoC)")
	}
	return nil
}

func fixupTxFx(tx *Tx, rl *fx.RateLoader) error {
	if tx.TxCurrency == DEFAULT_CURRENCY ||
		tx.TxCurrency == CAD {
		tx.TxCurrToLocalExchangeRate = 1.0
	}
	if tx.CommissionCurrency == DEFAULT_CURRENCY {
		tx.CommissionCurrency = tx.TxCurrency
	}

	if tx.TxCurrToLocalExchangeRate == 0.0 {
		if tx.TxCurrency != USD {
			return fmt.Errorf("Unsupported auto-FX for %s", tx.TxCurrency)
		}
		rate, err := rl.GetEffectiveUsdCadRate(tx.TradeDate)
		if err != nil {
			return err
		}
		tx.TxCurrToLocalExchangeRate = rate.ForeignToLocalRate
	}

	if tx.TxCurrency == tx.CommissionCurrency &&
		tx.CommissionCurrToLocalExchangeRate == 0.0 {
		// If this didn't get set, make it match the other.
		tx.CommissionCurrToLocalExchangeRate = tx.TxCurrToLocalExchangeRate
	} else if tx.CommissionCurrToLocalExchangeRate == 0.0 {
		if tx.TxCurrency != USD {
			return fmt.Errorf("Unsupported auto-FX for %s", tx.TxCurrency)
		}
		rate, err := rl.GetEffectiveUsdCadRate(tx.TradeDate)
		if err != nil {
			return err
		}
		tx.CommissionCurrToLocalExchangeRate = rate.ForeignToLocalRate
	}
	return nil
}

func ParseTxCsv(reader io.Reader, initialGlobalReadIndex uint32,
	csvDesc string, rateLoader *fx.RateLoader) ([]*Tx, error) {

	globalRowIndex := initialGlobalReadIndex
	csvR := csv.NewReader(reader)
	records, err := csvR.ReadAll()
	if err != nil {
		return nil, fmt.Errorf("Failed to parse CSV %s: %v", csvDesc, err)
	}

	if len(records) == 0 {
		return nil, fmt.Errorf("No rows found in %s", csvDesc)
	}

	header := records[0]

	colParsers := make([]ColParser, len(header))

	for i, col := range header {
		sanCol := strings.TrimSpace(strings.ToLower(col))
		if parser, ok := colParserMap[sanCol]; ok {
			colParsers[i] = parser
		} else {
			rateLoader.ErrPrinter.F("Warning: Unrecognized column %s\n", sanCol)
			colParsers[i] = parseNothing
		}
	}

	txs := make([]*Tx, 0, len(records)-1)
	for i, record := range records[1:] {
		tx := DefaultTx()
		tx.ReadIndex = globalRowIndex
		globalRowIndex++
		for j, col := range record {
			err = colParsers[j](strings.TrimSpace(col), tx)
			if err != nil {
				return nil, fmt.Errorf("Error parsing %s at line:col %d:%d: %v", csvDesc, i+1, j, err)
			}
		}
		err = CheckTxSanity(tx)
		if err != nil {
			return nil, fmt.Errorf("Error parsing %s at line %d: %v", csvDesc, i+1, err)
		}
		err = fixupTxFx(tx, rateLoader)
		if err != nil {
			return nil, err
		}
		txs = append(txs, tx)
	}
	return txs, nil
}

func parseNothing(data string, tx *Tx) error {
	return nil
}

func parseSecurity(data string, tx *Tx) error {
	tx.Security = strings.TrimSpace(data)
	return nil
}

func parseTradeDate(data string, tx *Tx) error {
	t, err := date.Parse(CsvDateFormat, data)
	if err != nil {
		return err
	}
	tx.TradeDate = t
	return nil
}

func parseSettlementDate(data string, tx *Tx) error {
	t, err := date.Parse(CsvDateFormat, data)
	if err != nil {
		return err
	}
	if tx.SettlementDate != (date.Date{}) {
		return fmt.Errorf(
			"Settlement Date provided twice (found both 'date' and 'settlement date' columns)")
	}
	tx.SettlementDate = t
	return nil
}

func parseAction(data string, tx *Tx) error {
	var action TxAction = NO_ACTION
	switch strings.TrimSpace(strings.ToLower(data)) {
	case "buy":
		action = BUY
	case "sell":
		action = SELL
	case "roc":
		action = ROC
	case "sfla":
		action = SFLA
	default:
		return fmt.Errorf("Invalid action: '%s'", data)
	}
	tx.Action = action
	return nil
}

func parseShares(data string, tx *Tx) error {
	shares, err := strconv.ParseUint(data, 10, 32)
	if err != nil {
		return fmt.Errorf("Error parsing # shares: %v", err)
	}
	tx.Shares = uint32(shares)
	return nil
}

func parseAmountPerShare(data string, tx *Tx) error {
	aps, err := strconv.ParseFloat(data, 64)
	if err != nil {
		return fmt.Errorf("Error parsing price/share: %v", err)
	}
	tx.AmountPerShare = aps
	return nil
}

func parseCommission(data string, tx *Tx) error {
	var c float64 = 0.0
	var err error
	if data != "" {
		c, err = strconv.ParseFloat(data, 64)
		if err != nil {
			return fmt.Errorf("Error parsing commission: %v", err)
		}
	}
	tx.Commission = c
	return nil
}

func parseTxCurr(data string, tx *Tx) error {
	tx.TxCurrency = Currency(strings.ToUpper(data))
	return nil
}

func parseTxFx(data string, tx *Tx) error {
	var fx float64 = 0.0
	var err error
	if data != "" {
		fx, err = strconv.ParseFloat(data, 64)
		if err != nil {
			return fmt.Errorf("Error parsing exchange rate: %v", err)
		}
	}
	tx.TxCurrToLocalExchangeRate = fx
	return nil
}

func parseCommissionCurr(data string, tx *Tx) error {
	tx.CommissionCurrency = Currency(strings.ToUpper(data))
	return nil
}

func parseCommissionFx(data string, tx *Tx) error {
	var fx float64 = 0.0
	var err error
	if data != "" {
		fx, err = strconv.ParseFloat(data, 64)
		if err != nil {
			return fmt.Errorf("Error parsing commission exchange rate: %v", err)
		}
	}
	tx.CommissionCurrToLocalExchangeRate = fx
	return nil
}

func parseSuperficialLoss(data string, tx *Tx) error {
	// Check for forcing marker (a terminating !)
	forceFlag := false
	if len(data) > 0 {
		forceFlag = data[len(data)-1] == '!'
		if forceFlag {
			data = data[:len(data)-1]
		}
	}

	sfl, err := strconv.ParseFloat(data, 64)
	if data != "" {
		if err != nil {
			return fmt.Errorf("Error parsing superficial loss: %v", err)
		}
		if sfl > 0.0 {
			return fmt.Errorf(
				"Error: superficial loss must be specified as a non-positive value: %f", sfl)
		}
		tx.SpecifiedSuperficialLoss = util.NewOptional[SFLInput](SFLInput{sfl, forceFlag})
	}
	return nil
}

func parseAffiliate(data string, tx *Tx) error {
	tx.Affiliate = GlobalAffiliateDedupTable.DedupedAffiliate(data)
	return nil
}

func parseMemo(data string, tx *Tx) error {
	tx.Memo = data
	return nil
}

func ToCsvString(txs []*Tx) string {
	var buf bytes.Buffer
	writer := csv.NewWriter(&buf)

	header := []string{
		"security",
		"trade date",
		"settlement date",
		"action",
		"shares",
		"amount/share",
		"commission",
		"currency",
		"exchange rate",
		"commission currency",
		"commission exchange rate",
		"superficial loss",
		"affiliate",
		"memo",
	}
	writer.Write(header)

	currString := func(curr Currency) string {
		if curr == DEFAULT_CURRENCY {
			return string(CAD)
		}
		return string(curr)
	}
	rateIsExplicit := func(curr Currency, rate float64) bool {
		if rate == 0.0 {
			return false
		} else if (curr == DEFAULT_CURRENCY || curr == CAD) && rate == 1.0 {
			return false
		}
		return true
	}

	for _, tx := range txs {
		txRate := ""
		commRate := ""
		if rateIsExplicit(tx.TxCurrency, tx.TxCurrToLocalExchangeRate) {
			txRate = fmt.Sprintf("%f", tx.TxCurrToLocalExchangeRate)
		}
		if rateIsExplicit(tx.CommissionCurrency, tx.CommissionCurrToLocalExchangeRate) {
			commRate = fmt.Sprintf("%f", tx.CommissionCurrToLocalExchangeRate)
		}
		sfl := ""
		if tx.SpecifiedSuperficialLoss.Present() {
			sflVal := tx.SpecifiedSuperficialLoss.MustGet()
			sfl = fmt.Sprintf("%f", sflVal.SuperficialLoss)
			if sflVal.Force {
				sfl += "!"
			}
		}

		record := []string{
			tx.Security,
			tx.TradeDate.String(),
			tx.SettlementDate.String(),
			tx.Action.String(),
			fmt.Sprintf("%d", tx.Shares),
			fmt.Sprintf("%f", tx.AmountPerShare),
			fmt.Sprintf("%f", tx.Commission),
			currString(tx.TxCurrency),
			txRate,
			currString(tx.CommissionCurrency),
			commRate,
			sfl,
			tx.Affiliate.Name(),
			tx.Memo,
		}
		writer.Write(record)
	}
	writer.Flush()

	return buf.String()
}
