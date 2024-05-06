package outfmt

import (
	"fmt"
	"io"

	"github.com/olekukonko/tablewriter"
	"github.com/tsiemens/acb/portfolio"
)

type STDWriter struct {
	w io.Writer
}

func NewSTDWriter(w io.Writer) *STDWriter {
	return &STDWriter{
		w: w,
	}
}

// Write implements io.Writer.
func (w *STDWriter) Write(p []byte) (int, error) {
	n, err := w.w.Write(p)
	if err != nil {
		panic(fmt.Errorf("STDWriter.Write: %w", err))
	}
	return n, err
}

// PrintRenderTable implements ACBWriter.
func (w *STDWriter) PrintRenderTable(outType OutputType, name string, tableModel *portfolio.RenderTable) error {
	for _, err := range tableModel.Errors {
		fmt.Fprintf(w, "[!] %v. Printing parsed information state:\n", err)
	}
	var title string
	switch outType {
	case Transactions:
		title = fmt.Sprintf("Transactions for %s", name)
	case AggregateGains:
		title = "Aggregate Gains"
	case Costs:
		title = fmt.Sprintf("%s Costs", name)
	default:
		panic(fmt.Sprint("OutputType ", outType, " is not implemented"))
	}
	fmt.Fprintf(w, "%s\n", title)

	table := tablewriter.NewWriter(w)
	table.SetHeader(tableModel.Header)
	table.SetBorder(false)
	table.SetRowLine(true)

	for _, row := range tableModel.Rows {
		table.Append(row)
	}

	table.SetFooter(tableModel.Footer)

	table.Render()

	for _, note := range tableModel.Notes {
		fmt.Fprintln(w, note)
	}

	fmt.Fprintln(w, "")
	return nil
}
