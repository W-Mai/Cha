package main

func Complex(x int) string {
	if x > 100 {
		return "big"
	} else if x > 50 {
		return "medium"
	}

	switch x {
	case 1:
		return "one"
	case 2:
		return "two"
	case 3:
		return "three"
	default:
		return "other"
	}
}
