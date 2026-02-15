package main

import (
	"fmt"
	"math/big"
	"sort"
	"strconv"
	"strings"
)

// CollatzTuple represents a single Collatz tuple
type CollatzTuple struct {
	Jump int
	K    int
	N    int
}

// TupleShape represents a zero-sum collection of tuples
type TupleShape struct {
	Tuples []CollatzTuple
	ID     int
}

func main() {
	maxN := 100
	fmt.Printf("Analyzing tuple shapes for first %d odd numbers\n", maxN)
	fmt.Println("=" + strings.Repeat("=", 60))

	// Compute jump and k values
	jumpValues := make([]int, maxN)
	kValues := make([]int, maxN)
	for i := 0; i < maxN; i++ {
		oddNum := 2*i + 1
		jumpValues[i], kValues[i] = getJumpAndK(oddNum)
	}

	// Print the sequence
	fmt.Println("\nJump sequence (first 50):")
	for i := 0; i < 50 && i < maxN; i++ {
		fmt.Printf("n=%d: jump=%d, k=%d\n", 2*i+1, jumpValues[i], kValues[i])
	}

	// Find all minimal zero-sum shapes
	fmt.Println("\n" + strings.Repeat("=", 60))
	fmt.Println("Computing minimal zero-sum shapes...")

	// Limit to 25 for subset enumeration (2^25 = 33M subsets)
	computeN := maxN
	if computeN > 25 {
		computeN = 25
	}

	allShapes := findMinimalZeroSumShapes(jumpValues[:computeN], kValues[:computeN])
	fmt.Printf("Found %d minimal zero-sum shapes\n", len(allShapes))

	// Group by k-pattern
	patterns := make(map[string][]TupleShape)
	for _, shape := range allShapes {
		pattern := getKPattern(shape)
		patterns[pattern] = append(patterns[pattern], shape)
	}

	// Sort patterns
	var patternKeys []string
	for k := range patterns {
		patternKeys = append(patternKeys, k)
	}
	sort.Strings(patternKeys)

	// PROGRESSIVE VIEW: Group shapes by birth n
	fmt.Println("\n" + strings.Repeat("=", 60))
	fmt.Println("PROGRESSIVE SHAPE EMERGENCE (when each shape is 'born'):")
	fmt.Println(strings.Repeat("-", 60))

	birthGroups := make(map[int][]TupleShape)
	for _, shape := range allShapes {
		birthN := getBirthN(shape)
		birthGroups[birthN] = append(birthGroups[birthN], shape)
	}

	// Sort by birth n
	var birthNs []int
	for n := range birthGroups {
		birthNs = append(birthNs, n)
	}
	sort.Ints(birthNs)

	cumulative := 0
	for _, n := range birthNs {
		shapes := birthGroups[n]
		cumulative += len(shapes)
		fmt.Printf("\nAt n=%d: %d new shapes born (total: %d)\n", n, len(shapes), cumulative)
		for _, shape := range shapes {
			fmt.Printf("  %s\n", formatShapeFull(shape))
		}
	}

	fmt.Println("\n" + strings.Repeat("=", 60))
	fmt.Println("SHAPES GROUPED BY K-PATTERN:")
	fmt.Println(strings.Repeat("-", 60))

	for _, pattern := range patternKeys {
		shapes := patterns[pattern]
		fmt.Printf("\nPattern %s: %d shapes\n", pattern, len(shapes))
		for _, shape := range shapes {
			fmt.Printf("  %s\n", formatShapeFull(shape))
		}
	}

	// Analyze which n values appear in shapes
	fmt.Println("\n" + strings.Repeat("=", 60))
	fmt.Println("SHAPES BY N VALUE (which shapes contain each n):")
	fmt.Println(strings.Repeat("-", 60))

	for i := 0; i < computeN; i++ {
		n := 2*i + 1
		var containingShapes []TupleShape
		for _, shape := range allShapes {
			for _, tuple := range shape.Tuples {
				if tuple.N == n {
					containingShapes = append(containingShapes, shape)
					break
				}
			}
		}
		if len(containingShapes) > 0 {
			fmt.Printf("\nn=%d (jump=%d, k=%d): %d shapes\n", n, jumpValues[i], kValues[i], len(containingShapes))
			for _, shape := range containingShapes {
				fmt.Printf("  %s\n", formatShapeCompact(shape))
			}
		}
	}

	// Analyze k-value distribution
	fmt.Println("\n" + strings.Repeat("=", 60))
	fmt.Println("K-VALUE DISTRIBUTION:")
	fmt.Println(strings.Repeat("-", 60))

	kCounts := make(map[int]int)
	for _, k := range kValues[:computeN] {
		kCounts[k]++
	}
	var ks []int
	for k := range kCounts {
		ks = append(ks, k)
	}
	sort.Ints(ks)
	for _, k := range ks {
		fmt.Printf("k=%d: %d occurrences\n", k, kCounts[k])
	}

	// Analyze shapes by size
	fmt.Println("\n" + strings.Repeat("=", 60))
	fmt.Println("SHAPES BY SIZE:")
	fmt.Println(strings.Repeat("-", 60))

	sizeCounts := make(map[int]int)
	for _, shape := range allShapes {
		sizeCounts[len(shape.Tuples)]++
	}
	var sizes []int
	for s := range sizeCounts {
		sizes = append(sizes, s)
	}
	sort.Ints(sizes)
	for _, s := range sizes {
		fmt.Printf("Size %d: %d shapes\n", s, sizeCounts[s])
	}

	// Find interesting patterns
	fmt.Println("\n" + strings.Repeat("=", 60))
	fmt.Println("INTERESTING OBSERVATIONS:")
	fmt.Println(strings.Repeat("-", 60))

	// Look for n values that don't appear in any shape
	fmt.Println("\nN values NOT in any shape:")
	for i := 0; i < computeN; i++ {
		n := 2*i + 1
		found := false
		for _, shape := range allShapes {
			for _, tuple := range shape.Tuples {
				if tuple.N == n {
					found = true
					break
				}
			}
			if found {
				break
			}
		}
		if !found {
			fmt.Printf("  n=%d (jump=%d, k=%d)\n", n, jumpValues[i], kValues[i])
		}
	}

	// Look for "central" n values (appear in many shapes)
	fmt.Println("\nMost frequently appearing n values:")
	nCounts := make(map[int]int)
	for _, shape := range allShapes {
		for _, tuple := range shape.Tuples {
			nCounts[tuple.N]++
		}
	}
	type nCount struct {
		n     int
		count int
	}
	var nCountList []nCount
	for n, c := range nCounts {
		nCountList = append(nCountList, nCount{n, c})
	}
	sort.Slice(nCountList, func(i, j int) bool {
		return nCountList[i].count > nCountList[j].count
	})
	for i := 0; i < 10 && i < len(nCountList); i++ {
		nc := nCountList[i]
		idx := (nc.n - 1) / 2
		fmt.Printf("  n=%d (jump=%d, k=%d): appears in %d shapes\n", nc.n, jumpValues[idx], kValues[idx], nc.count)
	}

	// Analyze complementary pairs
	fmt.Println("\nComplementary jump values (pairs that sum to 0):")
	jumpSet := make(map[int][]int) // jump -> list of n values
	for i := 0; i < computeN; i++ {
		jumpSet[jumpValues[i]] = append(jumpSet[jumpValues[i]], 2*i+1)
	}
	var jumps []int
	for j := range jumpSet {
		jumps = append(jumps, j)
	}
	sort.Ints(jumps)
	for _, j := range jumps {
		if j > 0 {
			if ns, ok := jumpSet[-j]; ok {
				fmt.Printf("  %d and %d: n=%v pairs with n=%v\n", j, -j, jumpSet[j], ns)
			}
		}
	}

	// Print raw data for further analysis
	fmt.Println("\n" + strings.Repeat("=", 60))
	fmt.Println("RAW DATA (CSV format):")
	fmt.Println("n,jump,k")
	for i := 0; i < maxN; i++ {
		fmt.Printf("%d,%d,%d\n", 2*i+1, jumpValues[i], kValues[i])
	}
}

func getJumpAndK(oddNum int) (int, int) {
	nBig := big.NewInt(int64(oddNum))
	three := big.NewInt(3)
	one := big.NewInt(1)

	result := new(big.Int).Mul(three, nBig)
	result.Add(result, one)

	k := findLargestPowerOf2(result)

	if k > 0 {
		powerOf2 := new(big.Int).Lsh(one, uint(k))
		result.Div(result, powerOf2)
	}

	if !result.IsInt64() {
		return 0, k
	}
	return int(result.Int64()) - oddNum, k
}

func findLargestPowerOf2(n *big.Int) int {
	if n.Sign() == 0 {
		return 0
	}
	k := 0
	temp := new(big.Int).Set(n)
	two := big.NewInt(2)
	zero := big.NewInt(0)
	mod := new(big.Int)

	for {
		mod.Mod(temp, two)
		if mod.Cmp(zero) != 0 {
			break
		}
		temp.Div(temp, two)
		k++
	}
	return k
}

func findMinimalZeroSumShapes(jumpValues, kValues []int) []TupleShape {
	var shapes []TupleShape
	n := len(jumpValues)

	shapeID := 0
	numSubsets := 1 << n
	for mask := 1; mask < numSubsets; mask++ {
		sum := 0
		for i := 0; i < n; i++ {
			if mask&(1<<i) != 0 {
				sum += jumpValues[i]
			}
		}
		if sum == 0 {
			if !hasProperZeroSumSubset(mask, n, jumpValues) {
				var tuples []CollatzTuple
				for i := 0; i < n; i++ {
					if mask&(1<<i) != 0 {
						tuples = append(tuples, CollatzTuple{
							Jump: jumpValues[i],
							K:    kValues[i],
							N:    2*i + 1,
						})
					}
				}
				shapes = append(shapes, TupleShape{Tuples: tuples, ID: shapeID})
				shapeID++
			}
		}
	}
	return shapes
}

func hasProperZeroSumSubset(mask int, n int, jumpValues []int) bool {
	for subMask := (mask - 1) & mask; subMask > 0; subMask = (subMask - 1) & mask {
		sum := 0
		for i := 0; i < n; i++ {
			if subMask&(1<<i) != 0 {
				sum += jumpValues[i]
			}
		}
		if sum == 0 {
			return true
		}
	}
	return false
}

func getKPattern(shape TupleShape) string {
	kVals := make([]int, len(shape.Tuples))
	for i, tuple := range shape.Tuples {
		kVals[i] = tuple.K
	}
	sort.Ints(kVals)

	result := "["
	for i, k := range kVals {
		if i > 0 {
			result += ","
		}
		result += strconv.Itoa(k)
	}
	result += "]"
	return result
}

func formatShapeCompact(shape TupleShape) string {
	result := "{"
	for i, tuple := range shape.Tuples {
		if i > 0 {
			result += ","
		}
		result += strconv.Itoa(tuple.Jump)
	}
	result += "}"
	return result
}

// getBirthN returns the n value at which this shape first becomes possible
func getBirthN(shape TupleShape) int {
	maxN := 0
	for _, tuple := range shape.Tuples {
		if tuple.N > maxN {
			maxN = tuple.N
		}
	}
	return maxN
}

func formatShapeFull(shape TupleShape) string {
	result := "{"
	for i, tuple := range shape.Tuples {
		if i > 0 {
			result += " "
		}
		result += fmt.Sprintf("(%d,k=%d,n=%d)", tuple.Jump, tuple.K, tuple.N)
	}
	result += "} @" + strconv.Itoa(getBirthN(shape))
	return result
}
