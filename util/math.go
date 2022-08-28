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

type Uint32Ratio struct {
	Numerator   uint32
	Denominator uint32
}

func (r *Uint32Ratio) Valid() bool {
	return r.Denominator != 0
}

func (r *Uint32Ratio) ToFloat64() float64 {
	return float64(r.Numerator) / float64(r.Denominator)
}
