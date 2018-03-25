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

var Verbose = false
var ForceDownload = false
var DoTest = false

func runRootCmd(cmd *cobra.Command, args []string) {
	// const tFmt = "2006-01-02 15:04:05"
	const tFmt = "2006-Jan-2"
	if len(args) > 0 {
		// CSV passed in
		csvName := args[0]
		err := ptf.ParseTxCsvFile(csvName)
		if err != nil {
			fmt.Println("Error:", err)
		}
		return
	}
	// t, err := time.Parse(tFmt, args[0])
	// if err != nil {
	// fmt.Fprintf(os.Stderr, "Error: %v\n", err)
	// os.Exit(1)
	// }
	// fmt.Printf("Time: %v\n", t)

	rates, err := fx.GetCadUsdRates(ForceDownload)
	if err != nil {
		fmt.Println("Error:", err)
	}
	for _, rate := range rates {
		fmt.Println(rate.String())
	}
}

func cmdName() string {
	binName := os.Args[0]
	return filepath.Base(binName)
}

// RootCmd represents the base command when called without any subcommands
var RootCmd = &cobra.Command{
	Use:   cmdName() + " [FILE]",
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
	Args: cobra.RangeArgs(0, 1),
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
