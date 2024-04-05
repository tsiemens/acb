package util

import (
	decimal "github.com/tsiemens/acb/decimal_value"
)

func MapKeys[K comparable, V any](m map[K]V) []K {
	keys := make([]K, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	return keys
}

func IntDecimalMapKeys(m map[int]decimal.Decimal) []int {
	return MapKeys[int, decimal.Decimal](m)
}

func IntFloat64MapKeys(m map[int]float64) []int {
	return MapKeys[int, float64](m)
}

type DefaultMap[K comparable, V any] struct {
	content     map[K]V
	defaultFunc func(K) V
}

func NewDefaultMap[K comparable, V any](defaultFunc func(K) V) *DefaultMap[K, V] {
	return &DefaultMap[K, V]{make(map[K]V), defaultFunc}
}

func (m *DefaultMap[K, V]) Get(key K) V {
	var val V
	var ok bool
	if val, ok = m.content[key]; !ok {
		val = m.defaultFunc(key)
		m.content[key] = val
	}
	return val
}

func (m *DefaultMap[K, V]) Set(key K, val V) {
	m.content[key] = val
}

func (m *DefaultMap[K, V]) EjectMap() map[K]V {
	content := m.content
	m.content = nil
	return content
}

func (m *DefaultMap[K, V]) Len() int {
	return len(m.content)
}

func (m *DefaultMap[K, V]) ForEach(fn func(K, V) bool) {
	for k, v := range m.content {
		if !fn(k, v) {
			break
		}
	}
}
