package cmd

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/spf13/cobra"
	// "github.com/spf13/viper"

	"github.com/tsiemens/acb/app"
	"github.com/tsiemens/acb/fx"
	"github.com/tsiemens/acb/log"
	ptf "github.com/tsiemens/acb/portfolio"
)

var ForceDownload = false
var InitialSymStatusOpt []string

var legacyOptions = app.NewLegacyOptions()

func runRootCmd(cmd *cobra.Command, args []string) {
	errPrinter := &log.StderrErrorPrinter{}

	allInitStatus, err := app.ParseInitialStatus(InitialSymStatusOpt)
	if err != nil {
		errPrinter.F("Error parsing --symbol-base: %v\n", err)
		os.Exit(1)
	}

	csvReaders := make([]app.DescribedReader, 0, len(args))
	for _, csvName := range args {
		fp, err := os.Open(csvName)
		if err != nil {
			errPrinter.F("Error: %v\n", err)
			os.Exit(1)
		}
		defer fp.Close()
		csvReaders = append(csvReaders, app.DescribedReader{csvName, fp})
	}

	ok := app.RunAcbAppToConsole(
		csvReaders, allInitStatus, ForceDownload,
		legacyOptions,
		&fx.CsvRatesCache{ErrPrinter: errPrinter}, errPrinter)
	if !ok {
		os.Exit(1)
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
	Version: "0.4.0",
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
	RootCmd.PersistentFlags().StringVar(&ptf.CsvDateFormat, "date-fmt", ptf.CsvDateFormatDefault,
		"Format of how dates appear in the csv file. Must represent Jan 2, 2006")
	RootCmd.Flags().StringSliceVarP(&InitialSymStatusOpt, "symbol-base", "b", []string{},
		"Base share count and ACBs for symbols, assumed at the beginning of time. "+
			"Formatted as SYM:nShares:totalAcb. Eg. GOOG:20:1000.00 . May be provided multiple times.")

	// Legacy Options
	RootCmd.PersistentFlags().BoolVar(&legacyOptions.NoSuperficialLosses,
		"legacy-no-superficial-losses", false,
		"Do not apply the superficial loss rule to sold shares (behaviour pre-v0.2).")
	RootCmd.PersistentFlags().BoolVar(&legacyOptions.SortBuysBeforeSells,
		"legacy-sort-buys-before-sells", false,
		"Sort all buys before all sells made on the same day (default behaviour pre-v0.4).")
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
