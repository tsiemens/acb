package portfolio

import (
	"encoding/csv"
	"fmt"
	"os"
	"strings"
)

func ParseTxCsvFile(fname string) error {
	fp, err := os.Open(fname)
	if err != nil {
		return err
	}
	defer fp.Close()

	csvR := csv.NewReader(fp)
	records, err := csvR.ReadAll()
	if err != nil {
		return err
	}
	for _, record := range records {
		fmt.Println(strings.Join(record, ","))
	}
	return nil
}
