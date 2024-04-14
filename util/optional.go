package util

import (
	"fmt"
)

type Optional[T any] struct {
	present bool
	value   T
}

func NewOptional[T any](v T) Optional[T] {
	return Optional[T]{true, v}
}

func (o *Optional[T]) Present() bool {
	return o.present
}

func (o *Optional[T]) Set(v T) {
	o.value = v
	o.present = true
}

func (o *Optional[T]) MustGet() T {
	if !o.present {
		panic("Optional.MustGet: value not present")
	}
	return o.value
}

func (o *Optional[T]) Get() (T, bool) {
	return o.value, o.present
}

func (o *Optional[T]) GetOr(orVal T) T {
	if o.present {
		return o.value
	}
	return orVal
}

// Returns (needValueCheck, equal)
func (o Optional[T]) NeedValueEqualityCheck(other Optional[T]) (bool, bool) {
	// Check presence match
	if o.present != other.present {
		return false, false
	}
	// If neither is present, they are equal
	if !o.present {
		return false, true
	}
	return true, false
}

// This cannot use a pointer receiver, likely because most of the time we don't
// have pointers to optionals. The pointer receiver will break the print formatter
// that's searching for Stringer interface satisfaction.
func (o Optional[T]) String() string {
	if o.present {
		return fmt.Sprintf("%v", o.value)
	}
	return "{empty}"
}
