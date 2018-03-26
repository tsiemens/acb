package portfolio

import (
	"encoding/csv"
	"fmt"
	"os"
	"strconv"
	"strings"
	"time"

	"github.com/tsiemens/acb/fx"
)

var CsvDateFormat string

type ColParser func(string, *Tx) error

var colParserMap = map[string]ColParser{
	"security":                 parseSecurity,
	"date":                     parseDate,
	"action":                   parseAction,
	"shares":                   parseShares,
	"amount/share":             parseAmountPerShare,
	"commission":               parseCommission,
	"currency":                 parseTxCurr,
	"exchange rate":            parseTxFx,
	"commission currency":      parseCommissionCurr,
	"commission exchange rate": parseCommissionFx,
	"memo": parseMemo,
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
		Security: "", Date: time.Time{}, Action: NO_ACTION,
		Shares: 0, AmountPerShare: 0.0, Commission: 0.0,
		TxCurrency: DEFAULT_CURRENCY, TxCurrToLocalExchangeRate: 0.0,
		CommissionCurrency: DEFAULT_CURRENCY, CommissionCurrToLocalExchangeRate: 0.0,
	}
}

func CheckTxSanity(tx *Tx) error {
	if tx.Security == "" {
		return fmt.Errorf("Transaction has no security")
	} else if (tx.Date == time.Time{}) {
		return fmt.Errorf("Transaction has no date")
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
		rate, err := rl.GetUsdCadRate(tx.Date)
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
		rate, err := rl.GetUsdCadRate(tx.Date)
		if err != nil {
			return err
		}
		tx.CommissionCurrToLocalExchangeRate = rate.ForeignToLocalRate
	}
	return nil
}

func ParseTxCsvFile(fname string, rateLoader *fx.RateLoader) ([]*Tx, error) {
	fp, err := os.Open(fname)
	if err != nil {
		return nil, err
	}
	defer fp.Close()

	csvR := csv.NewReader(fp)
	records, err := csvR.ReadAll()
	if err != nil {
		return nil, fmt.Errorf("Failed to parse CSV file %s: %v", fname, err)
	}

	if len(records) == 0 {
		return nil, fmt.Errorf("No rows found in %s", fname)
	}

	header := records[0]

	colParsers := make([]ColParser, len(header))

	for i, col := range header {
		sanCol := strings.TrimSpace(strings.ToLower(col))
		if parser, ok := colParserMap[sanCol]; ok {
			colParsers[i] = parser
		} else {
			fmt.Fprintf(os.Stderr, "Warning: Unrecognized column %s\n", sanCol)
			colParsers[i] = parseNothing
		}
	}

	txs := make([]*Tx, 0, len(records)-1)
	for i, record := range records[1:] {
		tx := DefaultTx()
		for j, col := range record {
			err = colParsers[j](col, tx)
			if err != nil {
				return nil, fmt.Errorf("Error parsing %s at line:col %d:%d: %v", fname, i+1, j, err)
			}
		}
		err = CheckTxSanity(tx)
		if err != nil {
			return nil, fmt.Errorf("Error parsing %s at line %d: %v", fname, i+1, err)
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
	tx.Security = data
	return nil
}

func parseDate(data string, tx *Tx) error {
	t, err := time.Parse(CsvDateFormat, data)
	if err != nil {
		return err
	}
	tx.Date = t
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

func parseMemo(data string, tx *Tx) error {
	tx.Memo = data
	return nil
}
