package util

// Now that we have generics, we can easily implement a ternary operator
// like most other languages have.
// i.e. in C++: x = cond ? a : b
func Tern[T any](cond bool, a T, b T) T {
	if cond {
		return a
	}
	return b
}
