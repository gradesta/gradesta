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

// KClassTrack represents a k-class arithmetic progression
type KClassTrack struct {
	K          int
	Formula    string // e.g., "(n+1)/2"
	NValues    []int  // which n values have this k
	JumpValues []int  // corresponding jumps
}

// PairingRule represents a pairing equation for zero-sum sets
type PairingRule struct {
	Pattern  string   // e.g., "[1,2]"
	Equation string   // e.g., "n2 = 2*n1 + 3"
	K1, K2   int      // the k values involved
	Pairs    [][2]int // pairs of (n1, n2) values
	Jumps    [][2]int // corresponding (jump1, jump2) values
}

// ZeroSumOriginsChapter shows where primitive zero-sum sets come from
type ZeroSumOriginsChapter struct {
	escConsumed bool

	// Data
	maxN     int
	tracks   []KClassTrack
	pairings []PairingRule

	// View state
	selectedTrack   int
	selectedPairing int
	scrollOffset    int

	// Help state
	helpState HelpDialogState
}

// NewZeroSumOriginsChapter creates a new Zero Sum Origins chapter
func NewZeroSumOriginsChapter() *ZeroSumOriginsChapter {
	z := &ZeroSumOriginsChapter{
		maxN:            50, // Start with 50 odd numbers
		selectedPairing: 0,
	}
	z.computeKClasses()
	z.computePairings()
	return z
}

// getJumpAndKForOddNumber calculates jump and k for an odd number
func (z *ZeroSumOriginsChapter) getJumpAndKForOddNumber(oddNum int) (int, int) {
	nBig := big.NewInt(int64(oddNum))
	three := big.NewInt(3)
	one := big.NewInt(1)

	result := new(big.Int).Mul(three, nBig)
	result.Add(result, one)

	k := findLargestPowerOf2Big(result)

	if k > 0 {
		powerOf2 := new(big.Int).Lsh(one, uint(k))
		result.Div(result, powerOf2)
	}

	if !result.IsInt64() {
		return 0, k
	}
	return int(result.Int64()) - oddNum, k
}

// computeKClasses groups n values by their k value and computes formulas
func (z *ZeroSumOriginsChapter) computeKClasses() {
	// First pass: collect all (n, jump, k) tuples
	kToData := make(map[int][][3]int) // k -> list of [n, jump, index]

	for i := 0; i < z.maxN; i++ {
		n := 2*i + 1
		jump, k := z.getJumpAndKForOddNumber(n)
		kToData[k] = append(kToData[k], [3]int{n, jump, i})
	}

	// Sort k values
	var ks []int
	for k := range kToData {
		ks = append(ks, k)
	}
	sort.Ints(ks)

	// Build tracks
	z.tracks = make([]KClassTrack, 0)
	for _, k := range ks {
		data := kToData[k]
		track := KClassTrack{
			K:          k,
			NValues:    make([]int, len(data)),
			JumpValues: make([]int, len(data)),
		}

		for i, d := range data {
			track.NValues[i] = d[0]
			track.JumpValues[i] = d[1]
		}

		// Derive formula based on k
		track.Formula = z.deriveFormula(k)

		z.tracks = append(z.tracks, track)
	}
}

// deriveFormula returns the jump formula for a given k
func (z *ZeroSumOriginsChapter) deriveFormula(k int) string {
	switch k {
	case 1:
		return "(n+1)/2"
	case 2:
		return "(1-n)/4"
	case 3:
		return "(1-5n)/8"
	case 4:
		return "(1-13n)/16"
	case 5:
		return "(1-29n)/32"
	case 6:
		return "(1-61n)/64"
	default:
		// General formula: (1 - (2^k - 3)*n) / 2^k
		coeff := (1 << k) - 3
		return "(1-" + strconv.Itoa(coeff) + "n)/" + strconv.Itoa(1<<k)
	}
}

// computePairings finds all size-2 zero-sum pairings
func (z *ZeroSumOriginsChapter) computePairings() {
	z.pairings = make([]PairingRule, 0)

	// Build lookup: jump value -> list of (n, k) pairs
	jumpToNK := make(map[int][][2]int)
	for _, track := range z.tracks {
		for i, n := range track.NValues {
			jump := track.JumpValues[i]
			jumpToNK[jump] = append(jumpToNK[jump], [2]int{n, track.K})
		}
	}

	// Find complementary pairs (jump1 + jump2 = 0)
	pairMap := make(map[string]*PairingRule) // pattern -> rule

	for jump, nkList := range jumpToNK {
		if jump <= 0 {
			continue // Only process positive jumps
		}
		negJump := -jump
		if negList, ok := jumpToNK[negJump]; ok {
			// Found complementary pair
			for _, nk1 := range nkList {
				for _, nk2 := range negList {
					n1, k1 := nk1[0], nk1[1]
					n2, k2 := nk2[0], nk2[1]

					// Create pattern key (sorted k values)
					var pattern string
					if k1 <= k2 {
						pattern = "[" + strconv.Itoa(k1) + "," + strconv.Itoa(k2) + "]"
					} else {
						pattern = "[" + strconv.Itoa(k2) + "," + strconv.Itoa(k1) + "]"
					}

					// Get or create rule
					rule, exists := pairMap[pattern]
					if !exists {
						rule = &PairingRule{
							Pattern:  pattern,
							K1:       min(k1, k2),
							K2:       max(k1, k2),
							Equation: z.derivePairingEquation(min(k1, k2), max(k1, k2)),
						}
						pairMap[pattern] = rule
					}

					// Add pair (always store k1's n first)
					if k1 <= k2 {
						rule.Pairs = append(rule.Pairs, [2]int{n1, n2})
						rule.Jumps = append(rule.Jumps, [2]int{jump, negJump})
					} else {
						rule.Pairs = append(rule.Pairs, [2]int{n2, n1})
						rule.Jumps = append(rule.Jumps, [2]int{negJump, jump})
					}
				}
			}
		}
	}

	// Convert map to sorted slice
	var patterns []string
	for p := range pairMap {
		patterns = append(patterns, p)
	}
	sort.Strings(patterns)

	for _, p := range patterns {
		rule := pairMap[p]
		// Sort pairs by first n value
		sort.Slice(rule.Pairs, func(i, j int) bool {
			return rule.Pairs[i][0] < rule.Pairs[j][0]
		})
		z.pairings = append(z.pairings, *rule)
	}
}

// derivePairingEquation returns the pairing equation for k1 and k2
func (z *ZeroSumOriginsChapter) derivePairingEquation(k1, k2 int) string {
	// For pattern [1,2]: n2 = 2*n1 + 3
	// For pattern [1,3]: n2 = (4*n1 + 5) / 5
	// etc.
	switch {
	case k1 == 1 && k2 == 2:
		return "n2 = 2*n1 + 3"
	case k1 == 1 && k2 == 3:
		return "n2 = (4*n1 + 5) / 5"
	case k1 == 1 && k2 == 4:
		return "n2 = (8*n1 + 13) / 13"
	case k1 == 1 && k2 == 6:
		return "n2 = (32*n1 + 61) / 61"
	case k1 == 2 && k2 == 3:
		return "n2 = (2*n1 - 3) / 5"
	default:
		return "complex"
	}
}

// Update updates the chapter
func (z *ZeroSumOriginsChapter) Update() error {
	z.escConsumed = false

	// Handle help dialog input
	if HandleHelpInput(&z.helpState, &z.escConsumed) {
		return nil
	}

	// Tab to cycle through pairings
	if inpututil.IsKeyJustPressed(ebiten.KeyTab) {
		if ebiten.IsKeyPressed(ebiten.KeyShift) {
			z.selectedPairing--
			if z.selectedPairing < 0 {
				z.selectedPairing = len(z.pairings) - 1
			}
		} else {
			z.selectedPairing++
			if z.selectedPairing >= len(z.pairings) {
				z.selectedPairing = 0
			}
		}
	}

	// Up/Down to select track
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowUp) {
		z.selectedTrack--
		if z.selectedTrack < 0 {
			z.selectedTrack = len(z.tracks) - 1
		}
	}
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowDown) {
		z.selectedTrack++
		if z.selectedTrack >= len(z.tracks) {
			z.selectedTrack = 0
		}
	}

	// Left/Right to scroll
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowLeft) {
		z.scrollOffset--
		if z.scrollOffset < 0 {
			z.scrollOffset = 0
		}
	}
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowRight) {
		z.scrollOffset++
	}

	// +/- to adjust maxN
	if inpututil.IsKeyJustPressed(ebiten.KeyEqual) || inpututil.IsKeyJustPressed(ebiten.KeyKPAdd) {
		z.maxN += 10
		if z.maxN > 100 {
			z.maxN = 100
		}
		z.computeKClasses()
		z.computePairings()
	}
	if inpututil.IsKeyJustPressed(ebiten.KeyMinus) || inpututil.IsKeyJustPressed(ebiten.KeyKPSubtract) {
		z.maxN -= 10
		if z.maxN < 20 {
			z.maxN = 20
		}
		z.computeKClasses()
		z.computePairings()
	}

	// R to reset
	if inpututil.IsKeyJustPressed(ebiten.KeyR) {
		z.scrollOffset = 0
		z.selectedTrack = 0
		z.selectedPairing = 0
	}

	return nil
}

// Draw draws the chapter
func (z *ZeroSumOriginsChapter) Draw(screen *ebiten.Image) {
	screen.Fill(color.RGBA{15, 15, 20, 255})

	// Title
	titleText := "Zero Sum Origins"
	titleBounds := text.BoundString(basicfont.Face7x13, titleText)
	titleX := (screenWidth - titleBounds.Dx()) / 2
	text.Draw(screen, titleText, basicfont.Face7x13, titleX, 20, color.White)

	subtitleText := "Where primitive zero-sum sets come from"
	subtitleBounds := text.BoundString(basicfont.Face7x13, subtitleText)
	subtitleX := (screenWidth - subtitleBounds.Dx()) / 2
	text.Draw(screen, subtitleText, basicfont.Face7x13, subtitleX, 35, color.Gray{Y: 150})

	// Layout
	sidebarWidth := 280
	trackAreaWidth := screenWidth - sidebarWidth - 30
	trackAreaX := 20
	trackAreaY := 60
	trackHeight := 40
	sidebarX := screenWidth - sidebarWidth - 10

	// Get selected pairing for highlighting
	var selectedRule *PairingRule
	if z.selectedPairing < len(z.pairings) {
		selectedRule = &z.pairings[z.selectedPairing]
	}

	// Draw k-class tracks
	z.drawTracks(screen, trackAreaX, trackAreaY, trackAreaWidth, trackHeight, selectedRule)

	// Draw sidebar with pairing equations
	z.drawPairingSidebar(screen, sidebarX, trackAreaY, sidebarWidth)

	// Draw help if open
	if z.helpState.ShowHelp {
		helpLines := z.getHelpLines()
		DrawHelpDialog(screen, helpLines, z.helpState.HelpScrollOffset)
	}

	// Status bar
	statusY := screenHeight - 30
	statusText := "Pattern: "
	if selectedRule != nil {
		statusText += selectedRule.Pattern + " | " + selectedRule.Equation + " | " + strconv.Itoa(len(selectedRule.Pairs)) + " pairs"
	}
	statusText += " | maxN: " + strconv.Itoa(z.maxN)
	text.Draw(screen, statusText, basicfont.Face7x13, 10, statusY, color.Gray{Y: 150})

	// Instructions
	instructions := "Tab: Patterns | Up/Down: Tracks | +/-: maxN | H: Help | ESC: Return"
	instBounds := text.BoundString(basicfont.Face7x13, instructions)
	instX := (screenWidth - instBounds.Dx()) / 2
	text.Draw(screen, instructions, basicfont.Face7x13, instX, screenHeight-12, color.Gray{Y: 100})
}

// drawTracks draws the k-class tracks with connecting lines
func (z *ZeroSumOriginsChapter) drawTracks(screen *ebiten.Image, x, y, width, trackHeight int, selectedRule *PairingRule) {
	// Build set of highlighted (n, k) pairs from selected rule
	highlightN := make(map[int]bool)
	if selectedRule != nil {
		for _, pair := range selectedRule.Pairs {
			highlightN[pair[0]] = true
			highlightN[pair[1]] = true
		}
	}

	// Calculate n-value spacing
	maxVisibleN := 30 // Show about 30 n values
	nSpacing := width / maxVisibleN
	if nSpacing < 20 {
		nSpacing = 20
	}

	// Draw each track
	for trackIdx, track := range z.tracks {
		trackY := y + trackIdx*trackHeight

		isSelectedTrack := trackIdx == z.selectedTrack

		// Draw track label
		labelText := "k=" + strconv.Itoa(track.K)
		var labelColor color.Color = color.Gray{Y: 180}
		if isSelectedTrack {
			labelColor = color.RGBA{255, 200, 100, 255}
		}
		text.Draw(screen, labelText, basicfont.Face7x13, x, trackY+15, labelColor)

		// Draw formula
		formulaText := track.Formula
		text.Draw(screen, formulaText, basicfont.Face7x13, x+35, trackY+15, color.Gray{Y: 120})

		// Draw track line
		lineY := trackY + 20
		lineStartX := x + 100
		for dx := 0; dx < width-100; dx++ {
			screen.Set(lineStartX+dx, lineY, color.Gray{Y: 40})
		}

		// Draw n values on track
		for i, n := range track.NValues {
			// Calculate x position based on n value
			nX := lineStartX + ((n-1)/2-z.scrollOffset)*nSpacing

			if nX < lineStartX || nX > x+width-20 {
				continue // Off screen
			}

			// Check if this n is part of selected pairing
			isHighlighted := highlightN[n]

			// Draw dot
			var dotColor color.Color = color.Gray{Y: 100}
			if isHighlighted {
				if track.JumpValues[i] > 0 {
					dotColor = color.RGBA{100, 255, 100, 255} // Green for positive
				} else {
					dotColor = color.RGBA{255, 100, 100, 255} // Red for negative
				}
			}

			for dy := -3; dy <= 3; dy++ {
				for dx := -3; dx <= 3; dx++ {
					if dx*dx+dy*dy <= 9 {
						screen.Set(nX+dx, lineY+dy, dotColor)
					}
				}
			}

			// Draw n label below if highlighted or every few values
			if isHighlighted || n <= 9 || n%10 == 1 {
				nLabel := strconv.Itoa(n)
				text.Draw(screen, nLabel, basicfont.Face7x13, nX-3, lineY+18, color.Gray{Y: 140})
			}

			// Draw jump value above if highlighted
			if isHighlighted {
				jumpLabel := strconv.Itoa(track.JumpValues[i])
				if track.JumpValues[i] > 0 {
					jumpLabel = "+" + jumpLabel
				}
				text.Draw(screen, jumpLabel, basicfont.Face7x13, nX-7, lineY-8, dotColor)
			}
		}
	}

	// Draw pairing lines between matched elements
	if selectedRule != nil {
		z.drawPairingLines(screen, x+100, y, nSpacing, trackHeight, selectedRule)
	}
}

// drawPairingLines draws lines connecting paired elements
func (z *ZeroSumOriginsChapter) drawPairingLines(screen *ebiten.Image, baseX, baseY, nSpacing, trackHeight int, rule *PairingRule) {
	// Find track indices for k1 and k2
	var track1Idx, track2Idx int
	for i, track := range z.tracks {
		if track.K == rule.K1 {
			track1Idx = i
		}
		if track.K == rule.K2 {
			track2Idx = i
		}
	}

	lineColor := color.RGBA{200, 150, 50, 150}

	for _, pair := range rule.Pairs {
		n1, n2 := pair[0], pair[1]

		// Calculate positions
		x1 := baseX + ((n1-1)/2-z.scrollOffset)*nSpacing
		y1 := baseY + track1Idx*trackHeight + 20

		x2 := baseX + ((n2-1)/2-z.scrollOffset)*nSpacing
		y2 := baseY + track2Idx*trackHeight + 20

		// Skip if off screen
		if x1 < baseX || x2 < baseX {
			continue
		}
		if x1 > screenWidth-100 || x2 > screenWidth-100 {
			continue
		}

		// Draw connecting line (simple straight line for now)
		z.drawLine(screen, x1, y1, x2, y2, lineColor)
	}
}

// drawLine draws a simple line between two points
func (z *ZeroSumOriginsChapter) drawLine(screen *ebiten.Image, x1, y1, x2, y2 int, c color.Color) {
	dx := x2 - x1
	dy := y2 - y1
	steps := max(abs(dx), abs(dy))
	if steps == 0 {
		return
	}

	for i := 0; i <= steps; i++ {
		t := float64(i) / float64(steps)
		x := x1 + int(float64(dx)*t)
		y := y1 + int(float64(dy)*t)
		screen.Set(x, y, c)
	}
}

// drawPairingSidebar draws the sidebar with pairing equations
func (z *ZeroSumOriginsChapter) drawPairingSidebar(screen *ebiten.Image, x, y, width int) {
	// Header
	text.Draw(screen, "Pairing Equations", basicfont.Face7x13, x, y, color.RGBA{255, 200, 100, 255})

	lineHeight := 14
	currentY := y + 20

	for i, rule := range z.pairings {
		if currentY > screenHeight-120 {
			break
		}

		isSelected := i == z.selectedPairing

		// Pattern header
		patternText := rule.Pattern
		if isSelected {
			patternText = "> " + patternText
		} else {
			patternText = "  " + patternText
		}
		patternText += ": " + rule.Equation

		var patternColor color.Color = color.Gray{Y: 180}
		if isSelected {
			patternColor = color.RGBA{255, 255, 100, 255}
		}
		text.Draw(screen, patternText, basicfont.Face7x13, x, currentY, patternColor)
		currentY += lineHeight

		// Show pairs if selected
		if isSelected {
			for j, pair := range rule.Pairs {
				if j >= 6 { // Limit shown pairs
					moreText := "    ..." + strconv.Itoa(len(rule.Pairs)-6) + " more"
					text.Draw(screen, moreText, basicfont.Face7x13, x, currentY, color.Gray{Y: 120})
					currentY += lineHeight
					break
				}
				if currentY > screenHeight-100 {
					break
				}

				// Show pair with jumps
				jump1, jump2 := rule.Jumps[j][0], rule.Jumps[j][1]
				pairText := "    n=" + strconv.Itoa(pair[0]) + "," + strconv.Itoa(pair[1])
				pairText += " -> {" + strconv.Itoa(jump1) + "," + strconv.Itoa(jump2) + "}"
				text.Draw(screen, pairText, basicfont.Face7x13, x, currentY, color.Gray{Y: 150})
				currentY += lineHeight
			}
		}
	}

	// Summary section
	summaryY := screenHeight - 100
	for dx := 0; dx < width; dx++ {
		screen.Set(x+dx, summaryY-10, color.Gray{Y: 50})
	}

	text.Draw(screen, "K-Class Formulas:", basicfont.Face7x13, x, summaryY, color.RGBA{150, 200, 255, 255})
	summaryY += lineHeight

	for i, track := range z.tracks {
		if i >= 4 {
			break
		}
		formulaText := "k=" + strconv.Itoa(track.K) + ": jump = " + track.Formula
		text.Draw(screen, formulaText, basicfont.Face7x13, x, summaryY, color.Gray{Y: 140})
		summaryY += lineHeight
	}
}

// getHelpLines returns help text
func (z *ZeroSumOriginsChapter) getHelpLines() []string {
	return []string{
		"",
		"ZERO SUM ORIGINS",
		"",
		"This chapter shows WHERE primitive zero-sum sets come from.",
		"",
		"K-CLASS ARITHMETIC PROGRESSIONS:",
		"  Each k value produces a distinct arithmetic progression:",
		"  k=1: +2, +4, +6, +8, +10, ... (all positive)",
		"  k=2: -2, -4, -6, -8, -10, ... (all negative)",
		"  k=3: -8, -18, -28, ... (negative, step -10)",
		"",
		"PAIRING EQUATIONS:",
		"  Zero-sum sets arise when values from different k-classes",
		"  satisfy specific algebraic equations.",
		"",
		"  Pattern [1,2]: n2 = 2*n1 + 3",
		"    Example: n1=3, n2=9 gives jumps +2 and -2",
		"",
		"  Pattern [1,3]: n2 = (4*n1 + 5) / 5",
		"    Only works when (4*n1 + 5) is divisible by 5",
		"",
		"CONTROLS:",
		"  Tab/Shift+Tab   - Cycle through pairing patterns",
		"  Up/Down         - Select k-class track",
		"  Left/Right      - Scroll n values",
		"  +/-             - Adjust maxN",
		"  R               - Reset view",
		"  H               - Show/hide help",
		"",
		"",
		"Press H or ESC to close",
	}
}

// WasEscConsumed returns whether ESC was consumed
func (z *ZeroSumOriginsChapter) WasEscConsumed() bool {
	return z.escConsumed
}

// Helper functions
func abs(x int) int {
	if x < 0 {
		return -x
	}
	return x
}

func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}

func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}
