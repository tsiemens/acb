package cmd

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/spf13/cobra"
	// "github.com/spf13/viper"

	"github.com/tsiemens/acb/fx"
	ptf "github.com/tsiemens/acb/portfolio"
)

const (
	CsvDateFormatDefault string = "2006-01-02"
)

var Verbose = false
var ForceDownload = false
var DoTest = false

func runRootCmd(cmd *cobra.Command, args []string) {
	rateLoader := fx.NewRateLoader(ForceDownload)

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
		deltas, err := ptf.TxsToDeltaList(secTxs, nil)
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
	Long: `A cli tool which can be used to perform Adjusted cost basis (ACB)
calculations on RSU and stock transactions.

Stocks and transactions can be in other currencies, and conversion rates for
certain currencies* can be automatically downloaded.

* Supported conversion rate pairs are:
 - CAD/USD`,
	// Uncomment the following line if your bare application
	// has an action associated with it:
	Run:  runRootCmd,
	Args: cobra.MinimumNArgs(1),
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
	RootCmd.PersistentFlags().BoolVarP(&Verbose, "verbose", "v", false,
		"Print verbose output")
	RootCmd.PersistentFlags().BoolVarP(&ForceDownload, "force-download", "f", false,
		"Download exchange rates, even if they are cached")
	RootCmd.PersistentFlags().StringVar(&ptf.CsvDateFormat, "date-fmt", CsvDateFormatDefault,
		"Format of how dates appear in the csv file. Must represent Jan 2, 2006")
	RootCmd.PersistentFlags().BoolVar(&DoTest, "test", false,
		"Do a test")
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
