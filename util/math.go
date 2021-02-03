package util

func MinUint32(val0 uint32, vals ...uint32) uint32 {
	min := val0
	for _, v := range vals {
		if v < min {
			min = v
		}
	}
	return min
}
