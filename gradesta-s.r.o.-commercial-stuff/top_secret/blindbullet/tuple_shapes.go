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

// CollatzTuple represents a single Collatz tuple
type CollatzTuple struct {
	Jump int // nextCollatz(n) - n
	K    int // power of 2 dividing (3n+1)
	N    int // the odd number
}

// TupleShape represents a zero-sum collection of tuples
type TupleShape struct {
	Tuples []CollatzTuple
	ID     int
}

// FilterMode determines additional filtering on top of progressive view
type FilterMode int

const (
	FilterNone FilterMode = iota // Progressive filtering only (default)
	FilterByK                    // Progressive + shapes containing specific k
)

// TupleShapesChapter implements the Tuple Shapes chapter
type TupleShapesChapter struct {
	escConsumed bool

	// Grid navigation (like jumps.go)
	selectedCol, selectedRow       int
	colScrollOffset, rowScrollOffset int
	visibleCols, visibleRows       int
	keyRepeatFrame                 int
	keyPressFrame                  map[ebiten.Key]int

	// Shape data
	maxN       int // Number of odd numbers to consider
	jumpValues []int
	kValues    []int
	allShapes  []TupleShape

	// Shape browsing
	selectedShapeIndex int
	shapeScrollOffset  int

	// Filtering
	filterMode   FilterMode
	filterKValue int
	filterNValue int

	// Pattern grouping (k-pattern -> shape indices)
	patterns    map[string][]int
	patternKeys []string

	// Help state
	helpState HelpDialogState
}

// NewTupleShapesChapter creates a new Tuple Shapes chapter
func NewTupleShapesChapter() *TupleShapesChapter {
	// Calculate visible columns and rows based on screen size
	cellWidth := 50
	cellHeight := 25
	colLabelWidth := 50
	rowLabelHeight := 30
	sidebarWidth := 250 // Width for shape catalog sidebar

	availableWidth := screenWidth - colLabelWidth - sidebarWidth - 20
	availableHeight := screenHeight - rowLabelHeight - 80 // Title + instructions + status bar

	visibleCols := availableWidth / cellWidth
	visibleRows := availableHeight / cellHeight

	if visibleCols < 1 {
		visibleCols = 1
	}
	if visibleRows < 1 {
		visibleRows = 1
	}

	t := &TupleShapesChapter{
		selectedCol:     0,
		selectedRow:     0,
		colScrollOffset: 0,
		rowScrollOffset: 0,
		visibleCols:     visibleCols,
		visibleRows:     visibleRows,
		keyPressFrame:   make(map[ebiten.Key]int),
		maxN:            20, // Start with 20 odd numbers
		patterns:        make(map[string][]int),
		filterMode:      FilterNone, // Progressive filtering by default
	}
	t.computeJumpValues()
	t.computeShapes()
	return t
}

// getJumpAndKForOddNumber calculates the jump difference and k value for an odd number
func (t *TupleShapesChapter) getJumpAndKForOddNumber(oddNum int) (int, int) {
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
		powerOf2 := new(big.Int).Lsh(one, uint(k))
		result.Div(result, powerOf2)
	}

	if !result.IsInt64() {
		return 0, k
	}
	return int(result.Int64()) - oddNum, k
}

// computeJumpValues computes jump and k values for all odd numbers up to maxN
func (t *TupleShapesChapter) computeJumpValues() {
	t.jumpValues = make([]int, t.maxN)
	t.kValues = make([]int, t.maxN)
	for i := 0; i < t.maxN; i++ {
		oddNum := 2*i + 1
		t.jumpValues[i], t.kValues[i] = t.getJumpAndKForOddNumber(oddNum)
	}
}

// hasProperZeroSumSubset checks if the given subset has any proper subset that sums to zero
func (t *TupleShapesChapter) hasProperZeroSumSubset(mask int, n int) bool {
	for subMask := (mask - 1) & mask; subMask > 0; subMask = (subMask - 1) & mask {
		sum := 0
		for i := 0; i < n; i++ {
			if subMask&(1<<i) != 0 {
				sum += t.jumpValues[i]
			}
		}
		if sum == 0 {
			return true
		}
	}
	return false
}

// getKPattern returns a sorted pattern string for k values in a shape
func (t *TupleShapesChapter) getKPattern(shape TupleShape) string {
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

// computeShapes computes all minimal zero-sum shapes
func (t *TupleShapesChapter) computeShapes() {
	t.allShapes = make([]TupleShape, 0)
	t.patterns = make(map[string][]int)
	t.patternKeys = make([]string, 0)

	n := len(t.jumpValues)
	if n == 0 {
		return
	}

	if n > maxSubsetElements {
		n = maxSubsetElements
	}

	shapeID := 0
	numSubsets := 1 << n
	for mask := 1; mask < numSubsets; mask++ {
		sum := 0
		for i := 0; i < n; i++ {
			if mask&(1<<i) != 0 {
				sum += t.jumpValues[i]
			}
		}
		if sum == 0 {
			// Check if minimal
			if !t.hasProperZeroSumSubset(mask, n) {
				// Create shape
				var tuples []CollatzTuple
				for i := 0; i < n; i++ {
					if mask&(1<<i) != 0 {
						tuples = append(tuples, CollatzTuple{
							Jump: t.jumpValues[i],
							K:    t.kValues[i],
							N:    2*i + 1,
						})
					}
				}
				shape := TupleShape{
					Tuples: tuples,
					ID:     shapeID,
				}
				t.allShapes = append(t.allShapes, shape)

				// Group by pattern
				pattern := t.getKPattern(shape)
				if _, exists := t.patterns[pattern]; !exists {
					t.patternKeys = append(t.patternKeys, pattern)
				}
				t.patterns[pattern] = append(t.patterns[pattern], shapeID)

				shapeID++
			}
		}
	}

	// Sort pattern keys
	sort.Strings(t.patternKeys)

	// Sort shapes by size, then by pattern
	sort.Slice(t.allShapes, func(i, j int) bool {
		if len(t.allShapes[i].Tuples) != len(t.allShapes[j].Tuples) {
			return len(t.allShapes[i].Tuples) < len(t.allShapes[j].Tuples)
		}
		return t.getKPattern(t.allShapes[i]) < t.getKPattern(t.allShapes[j])
	})
}

// getBirthN returns the n value at which this shape first becomes possible
// (the maximum n among all tuples in the shape)
func (t *TupleShapesChapter) getBirthN(shape TupleShape) int {
	maxN := 0
	for _, tuple := range shape.Tuples {
		if tuple.N > maxN {
			maxN = tuple.N
		}
	}
	return maxN
}

// getFilteredShapes returns shapes that are BORN at the current n value
// Only shows NEW shapes - those where birthN equals the selected n
func (t *TupleShapesChapter) getFilteredShapes() []TupleShape {
	// Get the current n value
	currentN := 2*t.selectedCol + 1

	var result []TupleShape
	for _, shape := range t.allShapes {
		// Only show shapes that are BORN at this exact n value
		birthN := t.getBirthN(shape)
		if birthN == currentN {
			// Additional filter by k if active
			if t.filterMode == FilterByK {
				hasK := false
				for _, tuple := range shape.Tuples {
					if tuple.K == t.filterKValue {
						hasK = true
						break
					}
				}
				if hasK {
					result = append(result, shape)
				}
			} else {
				result = append(result, shape)
			}
		}
	}
	return result
}

// Update updates the Tuple Shapes chapter
func (t *TupleShapesChapter) Update() error {
	t.escConsumed = false
	t.keyRepeatFrame++

	// Handle help dialog input
	if HandleHelpInput(&t.helpState, &t.escConsumed) {
		return nil
	}

	// Arrow key navigation with key repeat
	initialDelay := 20
	repeatDelay := 10

	moveCursor := func(deltaCol, deltaRow int) {
		oldCol := t.selectedCol
		t.selectedCol += deltaCol
		t.selectedRow += deltaRow

		if t.selectedCol < 0 {
			t.selectedCol = 0
		}
		if t.selectedRow < 0 {
			t.selectedRow = 0
		}

		// Update scroll offsets
		if t.selectedCol < t.colScrollOffset {
			t.colScrollOffset = t.selectedCol
		} else if t.selectedCol >= t.colScrollOffset+t.visibleCols {
			t.colScrollOffset = t.selectedCol - t.visibleCols + 1
		}

		if t.selectedRow < t.rowScrollOffset {
			t.rowScrollOffset = t.selectedRow
		} else if t.selectedRow >= t.rowScrollOffset+t.visibleRows {
			t.rowScrollOffset = t.selectedRow - t.visibleRows + 1
		}

		// Reset shape selection when column changes (progressive filtering is always active)
		if t.selectedCol != oldCol {
			t.selectedShapeIndex = 0
			t.shapeScrollOffset = 0
		}
	}

	arrowKeys := []ebiten.Key{ebiten.KeyArrowLeft, ebiten.KeyArrowRight, ebiten.KeyArrowUp, ebiten.KeyArrowDown}
	for _, key := range arrowKeys {
		if inpututil.IsKeyJustPressed(key) {
			t.keyPressFrame[key] = t.keyRepeatFrame
			switch key {
			case ebiten.KeyArrowLeft:
				moveCursor(-1, 0)
			case ebiten.KeyArrowRight:
				moveCursor(1, 0)
			case ebiten.KeyArrowUp:
				moveCursor(0, -1)
			case ebiten.KeyArrowDown:
				moveCursor(0, 1)
			}
		} else if ebiten.IsKeyPressed(key) {
			if pressFrame, ok := t.keyPressFrame[key]; ok {
				framesSincePress := t.keyRepeatFrame - pressFrame
				if framesSincePress >= initialDelay {
					if (framesSincePress-initialDelay)%repeatDelay == 0 {
						switch key {
						case ebiten.KeyArrowLeft:
							moveCursor(-1, 0)
						case ebiten.KeyArrowRight:
							moveCursor(1, 0)
						case ebiten.KeyArrowUp:
							moveCursor(0, -1)
						case ebiten.KeyArrowDown:
							moveCursor(0, 1)
						}
					}
				}
			}
		} else {
			delete(t.keyPressFrame, key)
		}
	}

	// Tab/Shift+Tab to cycle through shapes
	filteredShapes := t.getFilteredShapes()
	if inpututil.IsKeyJustPressed(ebiten.KeyTab) {
		if ebiten.IsKeyPressed(ebiten.KeyShift) {
			t.selectedShapeIndex--
			if t.selectedShapeIndex < 0 {
				t.selectedShapeIndex = len(filteredShapes) - 1
			}
		} else {
			t.selectedShapeIndex++
			if t.selectedShapeIndex >= len(filteredShapes) {
				t.selectedShapeIndex = 0
			}
		}
	}

	// PageUp/PageDown to scroll shape catalog
	if inpututil.IsKeyJustPressed(ebiten.KeyPageDown) {
		t.shapeScrollOffset += 5
	}
	if inpututil.IsKeyJustPressed(ebiten.KeyPageUp) {
		t.shapeScrollOffset -= 5
		if t.shapeScrollOffset < 0 {
			t.shapeScrollOffset = 0
		}
	}

	// K: Toggle filter by current row's k value (on top of progressive filtering)
	if inpututil.IsKeyJustPressed(ebiten.KeyK) {
		kValue := t.selectedRow + 1 // Row 0 = k=1
		if t.filterMode == FilterByK && t.filterKValue == kValue {
			// Toggle off k filter
			t.filterMode = FilterNone
		} else {
			t.filterMode = FilterByK
			t.filterKValue = kValue
		}
		t.selectedShapeIndex = 0
		t.shapeScrollOffset = 0
	}

	// C: Clear k filter (return to progressive-only filtering)
	if inpututil.IsKeyJustPressed(ebiten.KeyC) {
		t.filterMode = FilterNone
		t.selectedShapeIndex = 0
		t.shapeScrollOffset = 0
	}

	// +/-: Adjust maxN
	if inpututil.IsKeyJustPressed(ebiten.KeyEqual) || inpututil.IsKeyJustPressed(ebiten.KeyKPAdd) {
		t.maxN++
		if t.maxN > 25 {
			t.maxN = 25
		}
		t.computeJumpValues()
		t.computeShapes()
		t.selectedShapeIndex = 0
		t.shapeScrollOffset = 0
	}
	if inpututil.IsKeyJustPressed(ebiten.KeyMinus) || inpututil.IsKeyJustPressed(ebiten.KeyKPSubtract) {
		t.maxN--
		if t.maxN < 5 {
			t.maxN = 5
		}
		t.computeJumpValues()
		t.computeShapes()
		t.selectedShapeIndex = 0
		t.shapeScrollOffset = 0
	}

	// Enter: Jump grid to selected shape's first tuple
	if inpututil.IsKeyJustPressed(ebiten.KeyEnter) {
		if len(filteredShapes) > 0 && t.selectedShapeIndex < len(filteredShapes) {
			shape := filteredShapes[t.selectedShapeIndex]
			if len(shape.Tuples) > 0 {
				firstTuple := shape.Tuples[0]
				t.selectedCol = (firstTuple.N - 1) / 2 // n -> column index
				t.selectedRow = firstTuple.K - 1      // k -> row index

				// Update scroll
				if t.selectedCol < t.colScrollOffset {
					t.colScrollOffset = t.selectedCol
				} else if t.selectedCol >= t.colScrollOffset+t.visibleCols {
					t.colScrollOffset = t.selectedCol - t.visibleCols + 1
				}
				if t.selectedRow < t.rowScrollOffset {
					t.rowScrollOffset = t.selectedRow
				} else if t.selectedRow >= t.rowScrollOffset+t.visibleRows {
					t.rowScrollOffset = t.selectedRow - t.visibleRows + 1
				}
			}
		}
	}

	// R: Reset view
	if inpututil.IsKeyJustPressed(ebiten.KeyR) {
		t.selectedCol = 0
		t.selectedRow = 0
		t.colScrollOffset = 0
		t.rowScrollOffset = 0
		t.filterMode = FilterNone
		t.selectedShapeIndex = 0
		t.shapeScrollOffset = 0
	}

	return nil
}

// Draw draws the Tuple Shapes chapter
func (t *TupleShapesChapter) Draw(screen *ebiten.Image) {
	screen.Fill(color.RGBA{15, 15, 20, 255})

	// Draw title
	titleText := "Tuple Shapes"
	titleBounds := text.BoundString(basicfont.Face7x13, titleText)
	titleX := (screenWidth - titleBounds.Dx()) / 2
	titleY := 20
	text.Draw(screen, titleText, basicfont.Face7x13, titleX, titleY, color.White)

	// Draw subtitle
	subtitleText := "Zero-sum tuple shapes from Collatz sequence"
	subtitleBounds := text.BoundString(basicfont.Face7x13, subtitleText)
	subtitleX := (screenWidth - subtitleBounds.Dx()) / 2
	subtitleY := 35
	text.Draw(screen, subtitleText, basicfont.Face7x13, subtitleX, subtitleY, color.Gray{Y: 150})

	// Grid parameters
	cellWidth := 50
	cellHeight := 25
	colLabelWidth := 50
	rowLabelHeight := 30
	sidebarWidth := 250
	gridStartX := colLabelWidth + 10
	gridStartY := rowLabelHeight + 50

	// Calculate visible range
	startCol := t.colScrollOffset
	endCol := startCol + t.visibleCols
	if endCol > t.maxN {
		endCol = t.maxN
	}
	startRow := t.rowScrollOffset
	endRow := startRow + t.visibleRows

	// Get max k value from data
	maxK := 1
	for _, k := range t.kValues {
		if k > maxK {
			maxK = k
		}
	}
	if endRow > maxK+2 {
		endRow = maxK + 2
	}

	// Get selected shape for highlighting
	filteredShapes := t.getFilteredShapes()
	var selectedShape *TupleShape
	if len(filteredShapes) > 0 && t.selectedShapeIndex < len(filteredShapes) {
		selectedShape = &filteredShapes[t.selectedShapeIndex]
	}

	// Build set of highlighted cells with jump sign info
	// value: 1 = positive jump, -1 = negative jump, 0 = zero
	highlightedCells := make(map[string]int)
	if selectedShape != nil {
		for _, tuple := range selectedShape.Tuples {
			col := (tuple.N - 1) / 2
			row := tuple.K - 1
			key := strconv.Itoa(col) + "," + strconv.Itoa(row)
			if tuple.Jump > 0 {
				highlightedCells[key] = 1
			} else if tuple.Jump < 0 {
				highlightedCells[key] = -1
			} else {
				highlightedCells[key] = 0
			}
		}
	}

	// Shape highlight colors based on jump sign
	positiveColor := color.RGBA{80, 180, 80, 220}  // Green for positive
	negativeColor := color.RGBA{180, 80, 80, 220}  // Red for negative
	zeroColor := color.RGBA{180, 180, 80, 220}     // Yellow for zero

	// Draw column headers (odd numbers: 1, 3, 5, ...)
	for col := startCol; col < endCol; col++ {
		oddNum := 2*col + 1
		colX := gridStartX + (col-startCol)*cellWidth

		headerText := strconv.Itoa(oddNum)
		headerBounds := text.BoundString(basicfont.Face7x13, headerText)
		headerX := colX + (cellWidth-headerBounds.Dx())/2
		headerY := gridStartY - 5
		text.Draw(screen, headerText, basicfont.Face7x13, headerX, headerY, color.White)
	}

	// Draw row labels (k values: 1, 2, 3, ...)
	for row := startRow; row < endRow; row++ {
		rowY := gridStartY + (row-startRow)*cellHeight
		kValue := row + 1
		labelText := strconv.Itoa(kValue)
		labelBounds := text.BoundString(basicfont.Face7x13, labelText)
		labelX := colLabelWidth - labelBounds.Dx() - 5
		labelY := rowY + cellHeight - 5
		text.Draw(screen, labelText, basicfont.Face7x13, labelX, labelY, color.Gray{Y: 200})
	}

	// Draw cells
	for row := startRow; row < endRow; row++ {
		for col := startCol; col < endCol; col++ {
			cellX := gridStartX + (col-startCol)*cellWidth
			cellY := gridStartY + (row-startRow)*cellHeight

			isSelected := (col == t.selectedCol && row == t.selectedRow)
			cellKey := strconv.Itoa(col) + "," + strconv.Itoa(row)

			// Check if this cell is part of selected shape
			jumpSign, isHighlighted := highlightedCells[cellKey]

			// Draw cell background
			if isSelected {
				for dy := 0; dy < cellHeight-1; dy++ {
					for dx := 0; dx < cellWidth-1; dx++ {
						screen.Set(cellX+dx, cellY+dy, color.RGBA{100, 150, 255, 200})
					}
				}
			} else if isHighlighted {
				var highlightColor color.RGBA
				if jumpSign > 0 {
					highlightColor = positiveColor
				} else if jumpSign < 0 {
					highlightColor = negativeColor
				} else {
					highlightColor = zeroColor
				}
				for dy := 0; dy < cellHeight-1; dy++ {
					for dx := 0; dx < cellWidth-1; dx++ {
						screen.Set(cellX+dx, cellY+dy, highlightColor)
					}
				}
			}

			// Draw cell border
			borderColor := color.Gray{Y: 50}
			if isHighlighted {
				borderColor = color.Gray{Y: 150}
			}
			for dx := 0; dx < cellWidth-1; dx++ {
				screen.Set(cellX+dx, cellY, borderColor)
				screen.Set(cellX+dx, cellY+cellHeight-2, borderColor)
			}
			for dy := 0; dy < cellHeight-1; dy++ {
				screen.Set(cellX, cellY+dy, borderColor)
				screen.Set(cellX+cellWidth-2, cellY+dy, borderColor)
			}

			// Draw cell content (jump value if this is the natural k for this n)
			kValue := row + 1
			if col < len(t.kValues) && t.kValues[col] == kValue {
				jumpVal := t.jumpValues[col]
				cellText := strconv.Itoa(jumpVal)
				textBounds := text.BoundString(basicfont.Face7x13, cellText)
				textX := cellX + (cellWidth-textBounds.Dx())/2
				textY := cellY + cellHeight - 5

				var textColor color.Color
				if jumpVal > 0 {
					textColor = color.RGBA{100, 255, 100, 255}
				} else if jumpVal < 0 {
					textColor = color.RGBA{255, 100, 100, 255}
				} else {
					textColor = color.RGBA{255, 255, 100, 255}
				}
				text.Draw(screen, cellText, basicfont.Face7x13, textX, textY, textColor)
			}
		}
	}

	// Draw sidebar (shape catalog)
	sidebarX := screenWidth - sidebarWidth - 10
	t.drawShapeCatalog(screen, sidebarX, gridStartY, sidebarWidth)

	// Draw help dialog if open
	if t.helpState.ShowHelp {
		helpLines := t.getHelpLines()
		DrawHelpDialog(screen, helpLines, t.helpState.HelpScrollOffset)
	}

	// Draw status bar
	statusY := screenHeight - 30
	currentN := 2*t.selectedCol + 1
	statusText := "Born at n=" + strconv.Itoa(currentN) + " | " + strconv.Itoa(len(filteredShapes)) + " new shapes"
	if t.filterMode == FilterByK {
		statusText += " (k=" + strconv.Itoa(t.filterKValue) + " filter)"
	}
	statusText += " | maxN: " + strconv.Itoa(t.maxN)
	text.Draw(screen, statusText, basicfont.Face7x13, 10, statusY, color.Gray{Y: 150})

	// Draw instructions
	instructions := "Arrows: Navigate | Tab: Shapes | K: Filter k | C: Clear | +/-: maxN | H: Help | ESC: Return"
	instBounds := text.BoundString(basicfont.Face7x13, instructions)
	instX := (screenWidth - instBounds.Dx()) / 2
	instY := screenHeight - 12
	text.Draw(screen, instructions, basicfont.Face7x13, instX, instY, color.Gray{Y: 100})
}

// drawShapeCatalog draws the shape catalog sidebar
func (t *TupleShapesChapter) drawShapeCatalog(screen *ebiten.Image, x, y, width int) {
	// Header - show new shapes born at current n
	currentN := 2*t.selectedCol + 1
	headerText := "New at n=" + strconv.Itoa(currentN)
	if t.filterMode == FilterByK {
		headerText += " (k=" + strconv.Itoa(t.filterKValue) + ")"
	}
	text.Draw(screen, headerText, basicfont.Face7x13, x, y, color.RGBA{255, 200, 100, 255})

	filteredShapes := t.getFilteredShapes()
	lineHeight := 14
	currentY := y + 18

	// Group shapes by pattern
	patternShapes := make(map[string][]TupleShape)
	for _, shape := range filteredShapes {
		pattern := t.getKPattern(shape)
		patternShapes[pattern] = append(patternShapes[pattern], shape)
	}

	// Get sorted pattern keys
	var sortedPatterns []string
	for pattern := range patternShapes {
		sortedPatterns = append(sortedPatterns, pattern)
	}
	sort.Strings(sortedPatterns)

	// Draw pattern groups
	lineIdx := 0
	for _, pattern := range sortedPatterns {
		shapes := patternShapes[pattern]
		if lineIdx < t.shapeScrollOffset {
			lineIdx++
			continue
		}
		if currentY > screenHeight-80 {
			break
		}

		// Draw pattern header
		patternText := pattern + ": " + strconv.Itoa(len(shapes)) + " shapes"
		text.Draw(screen, patternText, basicfont.Face7x13, x, currentY, color.RGBA{150, 200, 255, 255})
		currentY += lineHeight
		lineIdx++

		// Draw shapes in this pattern
		for _, shape := range shapes {
			if lineIdx < t.shapeScrollOffset {
				lineIdx++
				continue
			}
			if currentY > screenHeight-80 {
				break
			}

			// Check if this is the selected shape
			isSelected := false
			if t.selectedShapeIndex < len(filteredShapes) {
				isSelected = shape.ID == filteredShapes[t.selectedShapeIndex].ID
			}

			// Format shape with birth n
			shapeText := "  "
			if isSelected {
				shapeText = "> "
			}
			shapeText += t.formatShapeCompact(shape)

			maxLen := width / 7
			if len(shapeText) > maxLen {
				shapeText = shapeText[:maxLen-2] + ".."
			}

			var shapeColor color.Color = color.Gray{Y: 180}
			if isSelected {
				shapeColor = color.RGBA{255, 255, 100, 255}
			}
			text.Draw(screen, shapeText, basicfont.Face7x13, x, currentY, shapeColor)
			currentY += lineHeight
			lineIdx++
		}
	}

	// Draw selected shape details
	if len(filteredShapes) > 0 && t.selectedShapeIndex < len(filteredShapes) {
		shape := filteredShapes[t.selectedShapeIndex]
		detailY := screenHeight - 80

		// Draw separator
		for dx := 0; dx < width; dx++ {
			screen.Set(x+dx, detailY-5, color.Gray{Y: 80})
		}

		text.Draw(screen, "Selected:", basicfont.Face7x13, x, detailY, color.RGBA{200, 200, 255, 255})
		detailY += lineHeight

		// Draw tuples
		tupleText := t.formatShapeTuples(shape)
		maxLen := width / 7
		if len(tupleText) > maxLen {
			tupleText = tupleText[:maxLen-2] + ".."
		}
		text.Draw(screen, tupleText, basicfont.Face7x13, x, detailY, color.White)
		detailY += lineHeight

		// Draw pattern
		patternText := "Pattern: " + t.getKPattern(shape)
		text.Draw(screen, patternText, basicfont.Face7x13, x, detailY, color.RGBA{150, 200, 255, 255})
	}
}

// formatShapeCompact returns a compact string representation of a shape
func (t *TupleShapesChapter) formatShapeCompact(shape TupleShape) string {
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

// formatShapeTuples returns a tuple representation of a shape
func (t *TupleShapesChapter) formatShapeTuples(shape TupleShape) string {
	result := "{"
	for i, tuple := range shape.Tuples {
		if i > 0 {
			result += " "
		}
		result += "(" + strconv.Itoa(tuple.Jump) + "," + strconv.Itoa(tuple.K) + "," + strconv.Itoa(tuple.N) + ")"
	}
	result += "}"
	return result
}

// getHelpLines returns the help text lines for the Tuple Shapes chapter
func (t *TupleShapesChapter) getHelpLines() []string {
	return []string{
		"",
		"TUPLE SHAPES - Progressive View",
		"",
		"This chapter visualizes zero-sum tuple shapes from the Collatz sequence.",
		"",
		"NEW SHAPES VIEW:",
		"  Shows only shapes BORN at the current n value.",
		"  A shape is 'born' when its highest n tuple becomes available.",
		"  Example: Shape {(2,n=3),(-2,n=9)} is born at n=9.",
		"  Navigate to see what's new at each n value.",
		"",
		"GRID:",
		"  Columns              - Odd numbers (1, 3, 5, 7, ...)",
		"  Rows                 - k values (power of 2 in Collatz step)",
		"  Cells                - Show jump value if this is the natural k for n",
		"  Green cells          - Positive jump (contributes + to sum)",
		"  Red cells            - Negative jump (contributes - to sum)",
		"",
		"NAVIGATION:",
		"  Arrow Keys           - Navigate grid (shapes update progressively)",
		"  Tab                  - Next shape",
		"  Shift+Tab            - Previous shape",
		"  PageUp/Down          - Scroll shape catalog",
		"  Enter                - Jump grid to selected shape's first tuple",
		"  R                    - Reset view",
		"",
		"FILTERING:",
		"  K                    - Additionally filter by current row's k value",
		"  C                    - Clear k filter",
		"",
		"SIZE:",
		"  +/=                  - Increase maxN (more odd numbers)",
		"  -                    - Decrease maxN (fewer odd numbers)",
		"",
		"SHAPE CATALOG:",
		"  Shows shapes grouped by k-pattern",
		"  Pattern [1,2] means one k=1 tuple + one k=2 tuple",
		"  @X shows when the shape is 'born' (its max n value)",
		"",
		"HELP:",
		"  H                    - Show/hide this help dialog",
		"  UP/DOWN or W/S       - Scroll help (when open)",
		"",
		"",
		"Press H or ESC to close",
	}
}

// WasEscConsumed returns whether ESC was consumed by a dialog this frame
func (t *TupleShapesChapter) WasEscConsumed() bool {
	return t.escConsumed
}
