package util

type Set[T comparable] struct {
	set map[T]bool
}

func NewSet[T comparable]() *Set[T] {
	return &Set[T]{make(map[T]bool)}
}

func (m *Set[T]) Has(val T) bool {
	_, ok := m.set[val]
	return ok
}

func (m *Set[T]) Add(val T) {
	m.set[val] = true
}
