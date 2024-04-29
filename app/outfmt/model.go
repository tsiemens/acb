package outfmt

import (
	"github.com/tsiemens/acb/portfolio"
)

type OutputType int

const (
	Transactions OutputType = iota
	AggregateGains
)

type ACBWriter interface {
	PrintRenderTable(outType OutputType, name string, tableModel *portfolio.RenderTable) error
}
