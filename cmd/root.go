package cmd

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"

	"github.com/spf13/cobra"
	// "github.com/spf13/viper"

	"github.com/tsiemens/acb/fx"
	"github.com/tsiemens/acb/log"
	ptf "github.com/tsiemens/acb/portfolio"
)

const (
	CsvDateFormatDefault string = "2006-01-02"
)

var ForceDownload = false
var InitialSymStatusOpt []string
var NoSuperficialLosses = false

func AllInitialStatus() (map[string]*ptf.PortfolioSecurityStatus, error) {
	stati := make(map[string]*ptf.PortfolioSecurityStatus)
	for _, opt := range InitialSymStatusOpt {
		parts := strings.Split(opt, ":")
		if len(parts) != 3 {
			return nil, fmt.Errorf("Invalid ACB format '%s'", opt)
		}
		symbol := parts[0]
		shares, err := strconv.ParseUint(parts[1], 10, 32)
		if err != nil {
			return nil, fmt.Errorf("Invalid shares format '%s'. %v", opt, err)
		}
		acb, err := strconv.ParseFloat(parts[2], 64)
		if err != nil {
			return nil, fmt.Errorf("Invalid ACB format '%s'. %v", opt, err)
		}

		if _, ok := stati[symbol]; ok {
			return nil, fmt.Errorf("Symbol %s specified multiple times", symbol)
		}
		stati[symbol] = &ptf.PortfolioSecurityStatus{
			Security: symbol, ShareBalance: uint32(shares), TotalAcb: acb}
	}
	return stati, nil
}

func runRootCmd(cmd *cobra.Command, args []string) {
	rateLoader := fx.NewRateLoader(ForceDownload)

	allInitStatus, err := AllInitialStatus()
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error parsing --symbol-base: %v\n", err)
		os.Exit(1)
	}

	allTxs := make([]*ptf.Tx, 0, 20)
	for i := 0; i < len(args); i++ {
		// CSV passed in
		csvName := args[i]
		txs, err := ptf.ParseTxCsvFile(csvName, rateLoader)
		if err != nil {
			fmt.Println("Error:", err)
			return
		}
		for _, tx := range txs {
			allTxs = append(allTxs, tx)
		}
	}

	allTxs = ptf.SortTxs(allTxs)
	txsBySec := ptf.SplitTxsBySecurity(allTxs)

	retVal := 0
	nSecs := len(txsBySec)
	i := 0
	for sec, secTxs := range txsBySec {
		secInitStatus, ok := allInitStatus[sec]
		if !ok {
			secInitStatus = nil
		}
		deltas, err := ptf.TxsToDeltaList(secTxs, secInitStatus, !NoSuperficialLosses)
		if err != nil {
			fmt.Printf("[!] %v. Printing parsed information state:\n", err)
			retVal = 1
		}
		fmt.Printf("Transactions for %s\n", sec)
		ptf.RenderTxTable(deltas)
		if i < (nSecs - 1) {
			fmt.Println("")
		}
		i++
	}
	if retVal != 0 {
		os.Exit(retVal)
	}
}

func cmdName() string {
	binName := os.Args[0]
	return filepath.Base(binName)
}

// RootCmd represents the base command when called without any subcommands
var RootCmd = &cobra.Command{
	Use:   cmdName() + " [CSV_FILE ...]",
	Short: "Adjusted cost basis (ACB) calculation tool",
	Long: fmt.Sprintf(
		`A cli tool which can be used to perform Adjusted cost basis (ACB)
calculations on RSU and stock transactions.

Stocks and transactions can be in other currencies, and conversion rates for
certain currencies* can be automatically downloaded or provided manually.

* Supported conversion rate pairs are:
 - CAD/USD

Each CSV provided should contain a header with these column names:
%s
Non-essential columns like exchange rates and currency columns are optional.

Exchange rates are always provided to be multiplied with the given amount to produce
the equivalent value in the default (local) currency.
 `, strings.Join(ptf.ColNames, ", ")),
	// Uncomment the following line if your bare application
	// has an action associated with it:
	Run:     runRootCmd,
	Args:    cobra.MinimumNArgs(1),
	Version: "0.2.0",
}

// Execute adds all child commands to the root command and sets flags appropriately.
// This is called by main.main(). It only needs to happen once to the rootCmd.
func Execute() {
	if err := RootCmd.Execute(); err != nil {
		fmt.Println(err)
		os.Exit(1)
	}
}

func init() {
	cobra.OnInitialize(onInit)

	// Persistent flags, which are global to the app cli
	RootCmd.PersistentFlags().BoolVarP(&log.VerboseEnabled, "verbose", "v", false,
		"Print verbose output")
	RootCmd.PersistentFlags().BoolVarP(&ForceDownload, "force-download", "f", false,
		"Download exchange rates, even if they are cached")
	RootCmd.PersistentFlags().StringVar(&ptf.CsvDateFormat, "date-fmt", CsvDateFormatDefault,
		"Format of how dates appear in the csv file. Must represent Jan 2, 2006")
	RootCmd.Flags().StringSliceVarP(&InitialSymStatusOpt, "symbol-base", "b", []string{},
		"Base share count and ACBs for symbols, assumed at the beginning of time. "+
			"Formatted as SYM:nShares:totalAcb. Eg. GOOG:20:1000.00 . May be provided multiple times.")
	RootCmd.PersistentFlags().BoolVar(&NoSuperficialLosses, "no-superficial-losses", false,
		"Do not apply the superficial loss rule to sold shares (behaviour pre-v0.2).")
}

// onInit reads in config file and ENV variables if set, and performs global
// or common actions before running command functions.
func onInit() {
	// if cfgFile != "" {
	//	 // Use config file from the flag.
	//	 viper.SetConfigFile(cfgFile)
	// } else {
	//	 // Find home directory.
	// // homedir "github.com/mitchellh/go-homedir"
	//	 home, err := homedir.Dir()
	//	 if err != nil {
	//		fmt.Println(err)
	//		os.Exit(1)
	//	}

	//	 // Search config in home directory with name ".acb-dummy" (without extension).
	//	 viper.AddConfigPath(home)
	//	 viper.SetConfigName(".acb-dummy")
	// }

	// viper.AutomaticEnv() // read in environment variables that match

	// // If a config file is found, read it in.
	// if err := viper.ReadInConfig(); err == nil {
	//	 fmt.Println("Using config file:", viper.ConfigFileUsed())
	// }
}
