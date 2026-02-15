package main

import (
	"image/color"
	"math/big"
	"sort"
	"strconv"

	"github.com/hajimehoshi/ebiten/v2"
	"github.com/hajimehoshi/ebiten/v2/inpututil"
	"github.com/hajimehoshi/ebiten/v2/text"
	"golang.org/x/image/font/basicfont"
)

// ZeroGenesisChapter implements the Zero Genesis chapter
// It visualizes zero-sum subsets from the jump sequence
type ZeroGenesisChapter struct {
	escConsumed bool

	// Current number of jumps to include (1 = first jump only)
	numJumps int

	// Cached jump values
	jumpValues []int

	// Cached k values (the power of 2 in the Collatz step)
	kValues []int

	// Cached delta values (change in zero-sum subset count when each number is added)
	deltaValues []int

	// Cached cumulative count of zero-sum subsets at each position
	countValues []int

	// Cached zero-sum subsets (each subset is a slice of indices into jumpValues)
	// Deduplicated by value multiset
	zeroSumSubsets [][]int

	// All minimal zero-sum subsets including "value duplicates" (different indices, same values)
	// These are unique when shown as (jump, k, n) tuples
	zeroSumSubsetsAll [][]int

	// Scroll offset for jump values (horizontal)
	jumpScrollOffset int

	// Scroll offset for subset list (vertical)
	subsetScrollOffset int

	// Help dialog state
	helpState HelpDialogState
}

// NewZeroGenesisChapter creates a new Zero Genesis chapter
func NewZeroGenesisChapter() *ZeroGenesisChapter {
	z := &ZeroGenesisChapter{
		numJumps:   1,
		jumpValues: make([]int, 0),
	}
	z.updateJumpValues()
	z.findZeroSumSubsets()
	z.updateDeltaValues()
	return z
}

// getJumpAndKForOddNumber calculates the jump difference and k value for an odd number
// Jump = nextCollatz(n) - n where nextCollatz(n) = (3n+1) / 2^k
// Returns (jump, k)
func (z *ZeroGenesisChapter) getJumpAndKForOddNumber(oddNum int) (int, int) {
	nBig := big.NewInt(int64(oddNum))
	three := big.NewInt(3)
	one := big.NewInt(1)

	// Calculate 3n + 1
	result := new(big.Int).Mul(three, nBig)
	result.Add(result, one)

	// Find k (number of times divisible by 2)
	k := findLargestPowerOf2Big(result)

	// Divide by 2^k to get the next number
	if k > 0 {
		powerOf2 := new(big.Int).Lsh(one, uint(k)) // 1 << k
		result.Div(result, powerOf2)
	}

	// Calculate jump difference
	if !result.IsInt64() {
		return 0, k // Overflow
	}
	return int(result.Int64()) - oddNum, k
}

// updateJumpValues updates the cached jump values and k values based on numJumps
func (z *ZeroGenesisChapter) updateJumpValues() {
	z.jumpValues = make([]int, z.numJumps)
	z.kValues = make([]int, z.numJumps)
	for i := 0; i < z.numJumps; i++ {
		oddNum := 2*i + 1 // Odd number: 1, 3, 5, 7, 9, ...
		z.jumpValues[i], z.kValues[i] = z.getJumpAndKForOddNumber(oddNum)
	}
}

// maxSubsetElements is the maximum number of elements we can efficiently enumerate subsets for
// 2^25 = 33 million subsets, which takes a few seconds but is manageable
const maxSubsetElements = 25

// hasProperZeroSumSubset checks if the given subset (represented as a bitmask) has any proper subset that sums to zero
func (z *ZeroGenesisChapter) hasProperZeroSumSubset(mask int, n int) bool {
	// Check all proper subsets of this mask
	// A proper subset has at least one bit removed
	for subMask := (mask - 1) & mask; subMask > 0; subMask = (subMask - 1) & mask {
		sum := 0
		for i := 0; i < n; i++ {
			if subMask&(1<<i) != 0 {
				sum += z.jumpValues[i]
			}
		}
		if sum == 0 {
			return true
		}
	}
	return false
}

// getSubsetValueKey returns a canonical string key for a subset's values (for deduplication)
func (z *ZeroGenesisChapter) getSubsetValueKey(indices []int) string {
	// Get the values and sort them to create a canonical representation
	values := make([]int, len(indices))
	for i, idx := range indices {
		values[i] = z.jumpValues[idx]
	}
	sort.Ints(values)

	// Convert to string key
	key := ""
	for i, v := range values {
		if i > 0 {
			key += ","
		}
		key += strconv.Itoa(v)
	}
	return key
}

// findZeroSumSubsets finds all minimal subsets of jumpValues that sum to zero
// A minimal zero-sum subset has no proper subset that also sums to zero
// Populates both deduplicated list and full list with all "value duplicates"
func (z *ZeroGenesisChapter) findZeroSumSubsets() {
	z.zeroSumSubsets = make([][]int, 0)
	z.zeroSumSubsetsAll = make([][]int, 0)

	n := len(z.jumpValues)
	if n == 0 {
		return
	}

	// For reasonable performance, limit elements
	if n > maxSubsetElements {
		n = maxSubsetElements
	}

	// Track seen value-multisets to avoid duplicates like {-4,4} and {4,-4}
	seenValueSets := make(map[string]bool)

	// Iterate through all 2^n - 1 non-empty subsets
	numSubsets := 1 << n
	for mask := 1; mask < numSubsets; mask++ {
		sum := 0
		subset := make([]int, 0)
		for i := 0; i < n; i++ {
			if mask&(1<<i) != 0 {
				sum += z.jumpValues[i]
				subset = append(subset, i)
			}
		}
		if sum == 0 {
			// Check if this is a minimal zero-sum subset (no proper subset sums to zero)
			if !z.hasProperZeroSumSubset(mask, n) {
				// Add to the full list (includes all "value duplicates")
				z.zeroSumSubsetsAll = append(z.zeroSumSubsetsAll, subset)

				// Add to deduplicated list only if we haven't seen these values
				valueKey := z.getSubsetValueKey(subset)
				if !seenValueSets[valueKey] {
					seenValueSets[valueKey] = true
					z.zeroSumSubsets = append(z.zeroSumSubsets, subset)
				}
			}
		}
	}

	// Sort both lists by size for better visualization
	sortFunc := func(subsets [][]int) {
		sort.Slice(subsets, func(i, j int) bool {
			if len(subsets[i]) != len(subsets[j]) {
				return len(subsets[i]) < len(subsets[j])
			}
			// If same size, sort by first element's value
			if len(subsets[i]) > 0 && len(subsets[j]) > 0 {
				return z.jumpValues[subsets[i][0]] < z.jumpValues[subsets[j][0]]
			}
			return false
		})
	}
	sortFunc(z.zeroSumSubsets)
	sortFunc(z.zeroSumSubsetsAll)
}

// hasProperZeroSumSubsetN checks if the given subset has any proper subset that sums to zero
// This version takes n as a parameter for the count function
func (z *ZeroGenesisChapter) hasProperZeroSumSubsetN(mask int, n int) bool {
	for subMask := (mask - 1) & mask; subMask > 0; subMask = (subMask - 1) & mask {
		sum := 0
		for i := 0; i < n; i++ {
			if subMask&(1<<i) != 0 {
				sum += z.jumpValues[i]
			}
		}
		if sum == 0 {
			return true
		}
	}
	return false
}

// getSubsetValueKeyN returns a canonical string key for a subset's values (for counting)
func (z *ZeroGenesisChapter) getSubsetValueKeyN(mask int, n int) string {
	values := make([]int, 0)
	for i := 0; i < n; i++ {
		if mask&(1<<i) != 0 {
			values = append(values, z.jumpValues[i])
		}
	}
	sort.Ints(values)

	key := ""
	for i, v := range values {
		if i > 0 {
			key += ","
		}
		key += strconv.Itoa(v)
	}
	return key
}

// countZeroSumSubsetsForN counts minimal zero-sum subsets for first n elements
// Deduplicates subsets with the same multiset of values
func (z *ZeroGenesisChapter) countZeroSumSubsetsForN(n int) int {
	if n == 0 || n > len(z.jumpValues) {
		return 0
	}

	if n > maxSubsetElements {
		n = maxSubsetElements
	}

	seenValueSets := make(map[string]bool)
	count := 0
	numSubsets := 1 << n
	for mask := 1; mask < numSubsets; mask++ {
		sum := 0
		for i := 0; i < n; i++ {
			if mask&(1<<i) != 0 {
				sum += z.jumpValues[i]
			}
		}
		if sum == 0 {
			// Only count if it's minimal (no proper subset sums to zero)
			if !z.hasProperZeroSumSubsetN(mask, n) {
				// Check for duplicate value sets
				valueKey := z.getSubsetValueKeyN(mask, n)
				if !seenValueSets[valueKey] {
					seenValueSets[valueKey] = true
					count++
				}
			}
		}
	}
	return count
}

// updateDeltaValues calculates the change in zero-sum subset count for each position
func (z *ZeroGenesisChapter) updateDeltaValues() {
	z.deltaValues = make([]int, z.numJumps)
	z.countValues = make([]int, z.numJumps)
	prevCount := 0
	for i := 1; i <= z.numJumps; i++ {
		currentCount := z.countZeroSumSubsetsForN(i)
		z.countValues[i-1] = currentCount
		z.deltaValues[i-1] = currentCount - prevCount
		prevCount = currentCount
	}
}

// Update updates the Zero Genesis chapter
func (z *ZeroGenesisChapter) Update() error {
	z.escConsumed = false

	// Handle help dialog input
	if HandleHelpInput(&z.helpState, &z.escConsumed) {
		return nil
	}

	// Handle arrow keys to change number of jumps
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowRight) || inpututil.IsKeyJustPressed(ebiten.KeyArrowUp) {
		z.numJumps++
		if z.numJumps > 50 { // Reasonable limit
			z.numJumps = 50
		}
		z.updateJumpValues()
		z.findZeroSumSubsets()
		z.updateDeltaValues()
		z.subsetScrollOffset = 0
	}
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowLeft) || inpututil.IsKeyJustPressed(ebiten.KeyArrowDown) {
		z.numJumps--
		if z.numJumps < 1 {
			z.numJumps = 1
		}
		z.updateJumpValues()
		z.findZeroSumSubsets()
		z.updateDeltaValues()
		z.subsetScrollOffset = 0
	}

	// Handle A/D for horizontal scrolling of jump values
	if inpututil.IsKeyJustPressed(ebiten.KeyD) {
		z.jumpScrollOffset += 5
		// Calculate max offset based on visible columns
		colWidth := 45
		labelWidth := 70
		startX := 20
		maxCols := (screenWidth - startX - labelWidth - 20) / colWidth
		maxOffset := len(z.jumpValues) - maxCols
		if maxOffset < 0 {
			maxOffset = 0
		}
		if z.jumpScrollOffset > maxOffset {
			z.jumpScrollOffset = maxOffset
		}
	}
	if inpututil.IsKeyJustPressed(ebiten.KeyA) {
		z.jumpScrollOffset -= 5
		if z.jumpScrollOffset < 0 {
			z.jumpScrollOffset = 0
		}
	}

	// Handle page up/down for scrolling subsets
	if inpututil.IsKeyJustPressed(ebiten.KeyPageDown) || inpututil.IsKeyJustPressed(ebiten.KeyJ) {
		z.subsetScrollOffset += 10
		maxOffset := len(z.zeroSumSubsets) - 10
		if maxOffset < 0 {
			maxOffset = 0
		}
		if z.subsetScrollOffset > maxOffset {
			z.subsetScrollOffset = maxOffset
		}
	}
	if inpututil.IsKeyJustPressed(ebiten.KeyPageUp) || inpututil.IsKeyJustPressed(ebiten.KeyK) {
		z.subsetScrollOffset -= 10
		if z.subsetScrollOffset < 0 {
			z.subsetScrollOffset = 0
		}
	}

	// Reset to initial state
	if inpututil.IsKeyJustPressed(ebiten.KeyR) {
		z.numJumps = 1
		z.jumpScrollOffset = 0
		z.subsetScrollOffset = 0
		z.updateJumpValues()
		z.findZeroSumSubsets()
		z.updateDeltaValues()
	}

	return nil
}

// Draw draws the Zero Genesis chapter
func (z *ZeroGenesisChapter) Draw(screen *ebiten.Image) {
	// Fill background
	screen.Fill(color.RGBA{15, 15, 20, 255})

	// Draw title
	titleText := "Zero Genesis"
	titleBounds := text.BoundString(basicfont.Face7x13, titleText)
	titleX := (screenWidth - titleBounds.Dx()) / 2
	titleY := 25
	text.Draw(screen, titleText, basicfont.Face7x13, titleX, titleY, color.White)

	// Draw subtitle/explanation
	subtitleText := "Minimal zero-sum subsets from the Collatz jump sequence"
	subtitleBounds := text.BoundString(basicfont.Face7x13, subtitleText)
	subtitleX := (screenWidth - subtitleBounds.Dx()) / 2
	subtitleY := 42
	text.Draw(screen, subtitleText, basicfont.Face7x13, subtitleX, subtitleY, color.Gray{Y: 150})

	// Draw the current jump values (the superset) - 5 rows now (n, Jump, k, Count, Delta)
	z.drawJumpValues(screen, 60)

	// Draw zero-sum subsets (adjusted for 5 rows of jump info)
	z.drawZeroSumSubsets(screen, 160)

	// Draw help dialog if open
	if z.helpState.ShowHelp {
		helpLines := z.getHelpLines()
		DrawHelpDialog(screen, helpLines, z.helpState.HelpScrollOffset)
	}

	// Draw instructions
	instructions := "LEFT/RIGHT: Add/remove | A/D: Scroll values | K/J: Scroll subsets | R: Reset | H: Help | ESC: Return"
	instBounds := text.BoundString(basicfont.Face7x13, instructions)
	instX := (screenWidth - instBounds.Dx()) / 2
	instY := screenHeight - 15
	text.Draw(screen, instructions, basicfont.Face7x13, instX, instY, color.Gray{Y: 120})
}

// drawJumpValues draws the current set of jump values, k values, and delta values
func (z *ZeroGenesisChapter) drawJumpValues(screen *ebiten.Image, startY int) {
	// Column width for aligned display
	colWidth := 45
	labelWidth := 70
	startX := 20

	// Calculate how many columns fit on screen
	maxCols := (screenWidth - startX - labelWidth - 20) / colWidth

	// Apply scroll offset
	startIdx := z.jumpScrollOffset
	endIdx := startIdx + maxCols
	if endIdx > len(z.jumpValues) {
		endIdx = len(z.jumpValues)
	}
	visibleCols := endIdx - startIdx

	// Draw header showing count and scroll position
	headerText := "(" + strconv.Itoa(z.numJumps) + " numbers)"
	if z.jumpScrollOffset > 0 || endIdx < len(z.jumpValues) {
		headerText += " [" + strconv.Itoa(startIdx+1) + "-" + strconv.Itoa(endIdx) + "]"
	}
	text.Draw(screen, headerText, basicfont.Face7x13, startX, startY, color.Gray{Y: 150})

	// Draw scroll arrows if applicable
	if z.jumpScrollOffset > 0 {
		text.Draw(screen, "<A", basicfont.Face7x13, startX+labelWidth-25, startY, color.RGBA{100, 200, 255, 255})
	}
	if endIdx < len(z.jumpValues) {
		arrowX := startX + labelWidth + visibleCols*colWidth
		text.Draw(screen, "D>", basicfont.Face7x13, arrowX, startY, color.RGBA{100, 200, 255, 255})
	}

	// Row 0: Index (odd number n)
	y := startY + 18
	text.Draw(screen, "n:", basicfont.Face7x13, startX, y, color.Gray{Y: 120})
	for i := 0; i < visibleCols; i++ {
		idx := startIdx + i
		oddNum := 2*idx + 1 // The odd number
		numStr := strconv.Itoa(oddNum)
		x := startX + labelWidth + i*colWidth
		text.Draw(screen, numStr, basicfont.Face7x13, x, y, color.Gray{Y: 120})
	}

	// Row 1: Jump values
	y += 16
	text.Draw(screen, "Jump:", basicfont.Face7x13, startX, y, color.RGBA{150, 200, 255, 255})
	for i := 0; i < visibleCols; i++ {
		idx := startIdx + i
		val := z.jumpValues[idx]
		numStr := strconv.Itoa(val)
		x := startX + labelWidth + i*colWidth

		// Color based on sign
		var numColor color.Color
		if val > 0 {
			numColor = color.RGBA{100, 255, 100, 255} // Green for positive
		} else if val < 0 {
			numColor = color.RGBA{255, 100, 100, 255} // Red for negative
		} else {
			numColor = color.RGBA{255, 255, 100, 255} // Yellow for zero
		}

		text.Draw(screen, numStr, basicfont.Face7x13, x, y, numColor)
	}

	// Row 2: k values
	y += 16
	text.Draw(screen, "k:", basicfont.Face7x13, startX, y, color.RGBA{200, 150, 255, 255})
	for i := 0; i < visibleCols; i++ {
		idx := startIdx + i
		kVal := z.kValues[idx]
		numStr := strconv.Itoa(kVal)
		x := startX + labelWidth + i*colWidth
		text.Draw(screen, numStr, basicfont.Face7x13, x, y, color.RGBA{200, 150, 255, 255})
	}

	// Row 3: Count values (cumulative zero-sum subset count)
	y += 16
	text.Draw(screen, "Count:", basicfont.Face7x13, startX, y, color.RGBA{255, 200, 100, 255})
	for i := 0; i < visibleCols; i++ {
		idx := startIdx + i
		count := z.countValues[idx]
		numStr := strconv.Itoa(count)
		x := startX + labelWidth + i*colWidth

		// Show "?" if beyond computation limit
		var countColor color.Color = color.RGBA{255, 200, 100, 255}
		if idx >= maxSubsetElements {
			numStr = strconv.Itoa(z.countValues[maxSubsetElements-1]) + "?"
			countColor = color.Gray{Y: 100}
		}

		text.Draw(screen, numStr, basicfont.Face7x13, x, y, countColor)
	}

	// Row 4: Delta values (change in zero-sum subset count)
	y += 16
	text.Draw(screen, "Delta:", basicfont.Face7x13, startX, y, color.RGBA{100, 200, 100, 255})
	for i := 0; i < visibleCols; i++ {
		idx := startIdx + i
		delta := z.deltaValues[idx]
		var numStr string
		if delta > 0 {
			numStr = "+" + strconv.Itoa(delta)
		} else {
			numStr = strconv.Itoa(delta)
		}
		x := startX + labelWidth + i*colWidth

		// Color based on delta, gray if beyond limit
		var deltaColor color.Color
		if idx >= maxSubsetElements {
			numStr = "?"
			deltaColor = color.Gray{Y: 100}
		} else if delta > 0 {
			deltaColor = color.RGBA{100, 255, 100, 255} // Green for increase
		} else if delta < 0 {
			deltaColor = color.RGBA{255, 100, 100, 255} // Red for decrease
		} else {
			deltaColor = color.Gray{Y: 120} // Gray for no change
		}

		text.Draw(screen, numStr, basicfont.Face7x13, x, y, deltaColor)
	}
}

// getSubsetsContainingLast returns subsets that contain the last (most recent) index
func (z *ZeroGenesisChapter) getSubsetsContainingLast() [][]int {
	lastIdx := z.numJumps - 1
	if lastIdx < 0 || lastIdx >= maxSubsetElements {
		lastIdx = maxSubsetElements - 1
	}

	result := make([][]int, 0)
	for _, subset := range z.zeroSumSubsets {
		for _, idx := range subset {
			if idx == lastIdx {
				result = append(result, subset)
				break
			}
		}
	}
	return result
}

// getSubsetsContainingLastAll returns all subsets (including value dups) containing the last index
func (z *ZeroGenesisChapter) getSubsetsContainingLastAll() [][]int {
	lastIdx := z.numJumps - 1
	if lastIdx < 0 || lastIdx >= maxSubsetElements {
		lastIdx = maxSubsetElements - 1
	}

	result := make([][]int, 0)
	for _, subset := range z.zeroSumSubsetsAll {
		for _, idx := range subset {
			if idx == lastIdx {
				result = append(result, subset)
				break
			}
		}
	}
	return result
}

// drawZeroSumSubsets draws the zero-sum subsets in three columns
func (z *ZeroGenesisChapter) drawZeroSumSubsets(screen *ebiten.Image, startY int) {
	// Get subsets containing the most recent value (deduplicated)
	newSubsets := z.getSubsetsContainingLast()

	// Three column layout
	col1X := 10
	col2X := screenWidth/3 + 5
	col3X := 2*screenWidth/3 + 5
	colWidth := screenWidth/3 - 15

	// Column 1 header: Deduplicated subsets
	header1 := "Unique values: " + strconv.Itoa(len(z.zeroSumSubsets))
	text.Draw(screen, header1, basicfont.Face7x13, col1X, startY, color.RGBA{255, 200, 100, 255})

	// Column 2 header: New deduplicated subsets
	lastVal := 0
	if z.numJumps > 0 && z.numJumps <= len(z.jumpValues) {
		lastIdx := z.numJumps - 1
		if lastIdx >= maxSubsetElements {
			lastIdx = maxSubsetElements - 1
		}
		lastVal = z.jumpValues[lastIdx]
	}
	header2 := "New(" + strconv.Itoa(lastVal) + "): " + strconv.Itoa(len(newSubsets))
	text.Draw(screen, header2, basicfont.Face7x13, col2X, startY, color.RGBA{100, 255, 200, 255})

	// Column 3 header: All subsets with tuples
	header3 := "Tuples(j,k,n): " + strconv.Itoa(len(z.zeroSumSubsetsAll))
	text.Draw(screen, header3, basicfont.Face7x13, col3X, startY, color.RGBA{255, 150, 255, 255})

	// Calculate visible lines
	y := startY + 18
	maxY := screenHeight - 40
	visibleLines := (maxY - y) / 14 // Slightly tighter spacing

	// Column 1: All deduplicated subsets
	startIdx := z.subsetScrollOffset
	endIdx := startIdx + visibleLines
	if endIdx > len(z.zeroSumSubsets) {
		endIdx = len(z.zeroSumSubsets)
	}

	drawY := y
	for i := startIdx; i < endIdx; i++ {
		subset := z.zeroSumSubsets[i]
		subsetStr := z.formatSubsetCompact(subset)

		maxLen := colWidth / 7
		if len(subsetStr) > maxLen {
			subsetStr = subsetStr[:maxLen-2] + ".."
		}

		var subsetColor color.Color
		if i%2 == 0 {
			subsetColor = color.RGBA{180, 180, 255, 255}
		} else {
			subsetColor = color.RGBA{180, 255, 180, 255}
		}

		text.Draw(screen, subsetStr, basicfont.Face7x13, col1X, drawY, subsetColor)
		drawY += 14
	}

	// Column 2: New subsets containing last value (deduplicated)
	drawY = y
	for i := 0; i < visibleLines && i < len(newSubsets); i++ {
		subset := newSubsets[i]
		subsetStr := z.formatSubsetCompact(subset)

		maxLen := colWidth / 7
		if len(subsetStr) > maxLen {
			subsetStr = subsetStr[:maxLen-2] + ".."
		}

		var subsetColor color.Color
		if i%2 == 0 {
			subsetColor = color.RGBA{100, 255, 200, 255}
		} else {
			subsetColor = color.RGBA{200, 255, 150, 255}
		}

		text.Draw(screen, subsetStr, basicfont.Face7x13, col2X, drawY, subsetColor)
		drawY += 14
	}

	// Column 3: All subsets with tuples (including "value duplicates")
	drawY = y
	tupleStartIdx := z.subsetScrollOffset
	tupleEndIdx := tupleStartIdx + visibleLines
	if tupleEndIdx > len(z.zeroSumSubsetsAll) {
		tupleEndIdx = len(z.zeroSumSubsetsAll)
	}

	for i := tupleStartIdx; i < tupleEndIdx; i++ {
		subset := z.zeroSumSubsetsAll[i]
		subsetStr := z.formatSubsetTuple(subset)

		maxLen := colWidth / 7
		if len(subsetStr) > maxLen {
			subsetStr = subsetStr[:maxLen-2] + ".."
		}

		var subsetColor color.Color
		if i%2 == 0 {
			subsetColor = color.RGBA{255, 150, 255, 255}
		} else {
			subsetColor = color.RGBA{255, 200, 200, 255}
		}

		text.Draw(screen, subsetStr, basicfont.Face7x13, col3X, drawY, subsetColor)
		drawY += 14
	}

	// Scroll indicators
	if len(z.zeroSumSubsets) > visibleLines {
		scrollInfo := "[" + strconv.Itoa(startIdx+1) + "-" + strconv.Itoa(endIdx) + "]"
		text.Draw(screen, scrollInfo, basicfont.Face7x13, col1X+colWidth-50, startY, color.Gray{Y: 100})
	}
	if len(z.zeroSumSubsetsAll) > visibleLines {
		scrollInfo := "[" + strconv.Itoa(tupleStartIdx+1) + "-" + strconv.Itoa(tupleEndIdx) + "]"
		text.Draw(screen, scrollInfo, basicfont.Face7x13, col3X+colWidth-50, startY, color.Gray{Y: 100})
	}
}

// formatSubset formats a subset as a string showing the actual values with sum, values sorted
func (z *ZeroGenesisChapter) formatSubset(indices []int) string {
	if len(indices) == 0 {
		return "{}"
	}

	// Get values and sort them for consistent display
	values := make([]int, len(indices))
	sum := 0
	for i, idx := range indices {
		values[i] = z.jumpValues[idx]
		sum += z.jumpValues[idx]
	}
	sort.Ints(values)

	result := "{"
	for i, v := range values {
		if i > 0 {
			result += ", "
		}
		result += strconv.Itoa(v)
	}
	result += "}"

	// Also show sum verification (should always be 0)
	result += " = " + strconv.Itoa(sum)

	return result
}

// formatSubsetCompact formats a subset compactly without the sum, values sorted
func (z *ZeroGenesisChapter) formatSubsetCompact(indices []int) string {
	if len(indices) == 0 {
		return "{}"
	}

	// Get values and sort them for consistent display
	values := make([]int, len(indices))
	for i, idx := range indices {
		values[i] = z.jumpValues[idx]
	}
	sort.Ints(values)

	result := "{"
	for i, v := range values {
		if i > 0 {
			result += ","
		}
		result += strconv.Itoa(v)
	}
	result += "}"

	return result
}

// formatSubsetTuple formats a subset as tuples (jump,k,n) - these are unique even for "value duplicates"
func (z *ZeroGenesisChapter) formatSubsetTuple(indices []int) string {
	if len(indices) == 0 {
		return "{}"
	}

	result := "{"
	for i, idx := range indices {
		if i > 0 {
			result += " "
		}
		jump := z.jumpValues[idx]
		k := z.kValues[idx]
		n := 2*idx + 1 // The odd number
		result += "(" + strconv.Itoa(jump) + "," + strconv.Itoa(k) + "," + strconv.Itoa(n) + ")"
	}
	result += "}"

	return result
}

// getHelpLines returns the help text lines for the Zero Genesis chapter
func (z *ZeroGenesisChapter) getHelpLines() []string {
	return []string{
		"",
		"ZERO GENESIS",
		"",
		"This chapter explores zero-sum subsets from the Collatz jump sequence.",
		"",
		"The jump sequence comes from the Collatz conjecture:",
		"  For each odd number n: jump = nextCollatz(n) - n",
		"  where nextCollatz(n) = (3n + 1) / 2^k",
		"  and k is the largest power of 2 dividing (3n + 1)",
		"",
		"First few jumps (for odd numbers 1, 3, 5, 7, 9, ...):",
		"  0, 2, -4, 4, -2, 10, -8, 8, -2, 14, ...",
		"",
		"DISPLAY ROWS:",
		"  n                   - The odd number (1, 3, 5, 7, ...)",
		"  Jump                - The jump value (nextCollatz(n) - n)",
		"  k                   - The power of 2 dividing (3n + 1)",
		"  Count               - Total zero-sum subsets up to this point",
		"  Delta               - Change in zero-sum subset count",
		"",
		"NOTE: Computation limited to first 25 values (2^25 subsets).",
		"      Beyond this limit, Count shows '?' and Delta shows '?'.",
		"",
		"NAVIGATION:",
		"  LEFT/DOWN           - Remove the last number from the set",
		"  RIGHT/UP            - Add the next jump number to the set",
		"  A                   - Scroll values left",
		"  D                   - Scroll values right",
		"  K/PgUp              - Scroll subsets up",
		"  J/PgDn              - Scroll subsets down",
		"  R                   - Reset to initial state (1 number)",
		"",
		"VISUALIZATION:",
		"  Green numbers       - Positive values / increases",
		"  Red numbers         - Negative values / decreases",
		"  Yellow numbers      - Zero",
		"  Purple numbers      - k values",
		"  Gray numbers        - Index (n) and no change (delta=0)",
		"",
		"  Only MINIMAL zero-sum subsets are shown.",
		"  (No proper subset of a shown subset also sums to zero)",
		"  Subsets are sorted by size (smallest first).",
		"",
		"HELP:",
		"  H                   - Show/hide this help dialog",
		"  UP/DOWN or W/S      - Scroll help (when open)",
		"",
		"",
		"Press H or ESC to close",
	}
}

// WasEscConsumed returns whether ESC was consumed by a dialog this frame
func (z *ZeroGenesisChapter) WasEscConsumed() bool {
	return z.escConsumed
}
