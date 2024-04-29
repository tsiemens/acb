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

func (m *Set[T]) AddAll(vals []T) {
	for _, v := range vals {
		m.Add(v)
	}
}

func (m *Set[T]) Len() int {
	return len(m.set)
}

func (m *Set[T]) ForEach(fn func(T) bool) {
	for k := range m.set {
		if !fn(k) {
			break
		}
	}
}

func (m *Set[T]) ToSlice() []T {
	slice := make([]T, 0, len(m.set))
	for k := range m.set {
		slice = append(slice, k)
	}
	return slice
}
