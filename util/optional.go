package util

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

func (o *Optional[T]) MustGet() T {
	if !o.present {
		panic("Optional.MustGet: value not present")
	}
	return o.value
}
