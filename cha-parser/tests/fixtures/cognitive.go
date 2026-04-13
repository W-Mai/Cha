package main

// CognitiveComplexity should be:
// for: +1
//   for: +2 (nesting=1)
//     if: +3 (nesting=2)
//       continue: +1 (label)
// Total = 7
func SumOfPrimes(max int) int {
	var total int
OUT:
	for i := 1; i < max; i++ {
		for j := 2; j < i; j++ {
			if i%j == 0 {
				continue OUT
			}
		}
		total += i
	}
	return total
}
