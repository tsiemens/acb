package outfmt

import (
	"encoding/csv"
	"fmt"
	"os"
	"path"

	"github.com/tsiemens/acb/portfolio"
)

type CSVWriter struct {
	OutDir string
}

// PrintRenderTable implements ACBWriter.
func (w *CSVWriter) PrintRenderTable(outType OutputType, name string, tableModel *portfolio.RenderTable) error {
	var fn string
	switch outType {
	case Transactions:
		fn = fmt.Sprintf("%s.csv", name)
	case AggregateGains:
		fn = "aggregate-gains.csv"
	default:
		return fmt.Errorf("OutputType %v not implemented", outType)
	}

	fp, err := os.Create(path.Join(w.OutDir, fn))
	if err != nil {
		return fmt.Errorf("Create file %q: %w", fn, err)
	}
	defer fp.Close()

	csvWriter := csv.NewWriter(fp)

	if err := csvWriter.Write(tableModel.Header); err != nil {
		return fmt.Errorf("write header: %w", err)
	}

	for _, row := range tableModel.Rows {
		if err := csvWriter.Write(row); err != nil {
			return fmt.Errorf("write row: %w", err)
		}
	}
	if len(tableModel.Footer) > 0 {
		if err := csvWriter.Write(tableModel.Footer); err != nil {
			return fmt.Errorf("write footer: %w", err)
		}
	}
	csvWriter.Flush()

	for _, note := range tableModel.Notes {
		fmt.Fprintln(fp, note)
	}

	return nil
}

func NewCSVWriter(outDir string) (*CSVWriter, error) {
	if err := os.MkdirAll(outDir, os.ModePerm); err != nil {
		return nil, fmt.Errorf("Creating CSV output directory: %w", err)
	}
	return &CSVWriter{OutDir: outDir}, nil
}
