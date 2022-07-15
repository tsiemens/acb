package test

import (
	// "fmt"
	"os"
	"testing"
)

type TestContext struct {
	UseLegacyCsvHeaders bool
	CsvHeaders          string
}

var ctx TestContext

func setup() {
	ctx = TestContext{UseLegacyCsvHeaders: false, CsvHeaders: ""}
	// Do something here.

	// fmt.Printf("\033[1;36m%s\033[0m", "> Setup completed\n")
}

func teardown() {
	// Do something here if necessary.

	// fmt.Printf("\033[1;36m%s\033[0m", "> Teardown completed")
	// fmt.Printf("\n")
}

func resetContext() {
	setup()
}

func TestMain(m *testing.M) {
	setup()
	code := m.Run()
	teardown()
	os.Exit(code)
}
