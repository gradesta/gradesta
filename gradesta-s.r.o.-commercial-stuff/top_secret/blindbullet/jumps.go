package main

import (
	"image/color"
	"math/big"
	"strconv"

	"github.com/hajimehoshi/ebiten/v2"
	"github.com/hajimehoshi/ebiten/v2/inpututil"
	"github.com/hajimehoshi/ebiten/v2/text"
	"golang.org/x/image/font/basicfont"
)

// JumpsChapter implements the Jumps chapter
type JumpsChapter struct {
	escConsumed bool

	// Selected cell position
	selectedCol int // Column index (odd number index: -3, -2, -1, 0, 1, 2, 3...)
	selectedRow int // Row index (k value: 1, 2, 3, ...) - note: row 0 = k=1

	// Scroll offsets
	colScrollOffset int // Horizontal scroll offset (column index)
	rowScrollOffset int // Vertical scroll offset (row index, where row 0 = k=1)

	// Visible area
	visibleCols int
	visibleRows int

	// Key repeat
	keyRepeatFrame int
	keyPressFrame  map[ebiten.Key]int // Track when each key was first pressed

	// Display mode
	displayMode int // 0 = Normal (k value), 1 = Jump (difference), 2 = Index (destination)
}

// NewJumpsChapter creates a new Jumps chapter
func NewJumpsChapter() *JumpsChapter {
	// Calculate visible columns and rows based on screen size
	cellWidth := 80
	cellHeight := 25
	colLabelWidth := 100
	rowLabelHeight := 30
	fixedColWidth := 100 // Width for fixed column on right

	availableWidth := screenWidth - colLabelWidth - fixedColWidth - 20
	availableHeight := screenHeight - rowLabelHeight - 60 // Title + instructions

	visibleCols := availableWidth / cellWidth
	visibleRows := availableHeight / cellHeight

	if visibleCols < 1 {
		visibleCols = 1
	}
	if visibleRows < 1 {
		visibleRows = 1
	}

	return &JumpsChapter{
		selectedCol:     0, // Start at column 0 (odd number 1)
		selectedRow:     0, // Start at row 0 (k=1)
		colScrollOffset: 0,
		rowScrollOffset: 0,
		visibleCols:    visibleCols,
		visibleRows:    visibleRows,
		keyPressFrame:  make(map[ebiten.Key]int),
		displayMode:    0, // Start in Normal mode
	}
}

// getOddNumberForCol returns the odd number for a given column index
func (j *JumpsChapter) getOddNumberForCol(colIndex int) int {
	return 2*colIndex + 1
}

// getFirstKValue calculates the first k value for an odd number
func (j *JumpsChapter) getFirstKValue(oddNum int) int {
	nBig := big.NewInt(int64(oddNum))
	three := big.NewInt(3)
	one := big.NewInt(1)

	// Calculate 3n + 1
	result := new(big.Int).Mul(three, nBig)
	result.Add(result, one)

	// Find k (number of times divisible by 2)
	k := findLargestPowerOf2Big(result)
	return k
}

// getCollatzNext calculates the next odd number in the Collatz sequence
// Returns the result as a big.Int to avoid overflow
func (j *JumpsChapter) getCollatzNextBig(oddNum int) *big.Int {
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

	return result
}

// getCollatzNext calculates the next odd number in the Collatz sequence
func (j *JumpsChapter) getCollatzNext(oddNum int) int {
	result := j.getCollatzNextBig(oddNum)
	// Check if result fits in int64
	if !result.IsInt64() {
		// Overflow - return 0 or handle appropriately
		return 0
	}
	return int(result.Int64())
}

// getJumpDifference calculates the difference between current index and next Collatz index
func (j *JumpsChapter) getJumpDifference(oddNum int) int {
	nextOdd := j.getCollatzNext(oddNum)
	// The difference is: nextOdd - oddNum
	return nextOdd - oddNum
}

// getIncomingJumpSource finds the source index that jumps to the given destination with the given k value
// Returns the source index if found, 0 if no incoming jump exists
// For a destination n and k value k, we need to find m such that:
//   (3m + 1) / 2^k = n
//   This means: 3m + 1 = n * 2^k
//   So: m = (n * 2^k - 1) / 3
// For this to be valid:
//   - n * 2^k - 1 must be divisible by 3
//   - m must be odd
//   - The k value for m must be k (i.e., when we apply Collatz to m, we get k divisions by 2)
func (j *JumpsChapter) getIncomingJumpSource(destOddNum int, kValue int) int {
	// Calculate n * 2^k - 1
	nBig := big.NewInt(int64(destOddNum))
	powerOf2 := big.NewInt(1)
	powerOf2.Lsh(powerOf2, uint(kValue)) // 2^k
	
	nTimesPowerOf2 := new(big.Int).Mul(nBig, powerOf2)
	nTimesPowerOf2Minus1 := new(big.Int).Sub(nTimesPowerOf2, big.NewInt(1))
	
	// Check if divisible by 3
	three := big.NewInt(3)
	mod := new(big.Int).Mod(nTimesPowerOf2Minus1, three)
	if mod.Sign() != 0 {
		// Not divisible by 3, no incoming jump
		return 0
	}
	
	// Calculate m = (n * 2^k - 1) / 3
	sourceBig := new(big.Int).Div(nTimesPowerOf2Minus1, three)
	
	// Check if m fits in int64
	if !sourceBig.IsInt64() {
		return 0
	}
	
	source := int(sourceBig.Int64())
	
	// Check if source is odd
	if source%2 == 0 {
		return 0
	}
	
	// Verify that the k value for source is actually kValue
	// (i.e., when we apply Collatz to source, we get kValue divisions by 2)
	actualK := j.getFirstKValue(source)
	if actualK != kValue {
		return 0
	}
	
	return source
}

// getFixedColumnValue returns the value for the fixed column at a given row
// In Normal/Jump modes: shows jump differences
// In Index mode: shows destination index differences
// Returns (value, found) where found indicates if we found actual values (true) or used pattern/fallback (false)
func (j *JumpsChapter) getFixedColumnValue(row int) (int, bool) {
	if j.displayMode == 2 {
		// Index mode: calculate destination index difference for this k value
		return j.getDestinationIndexDifference(row + 1) // row 0 = k=1
	}
	
	// Normal/Jump modes: show jump differences
	switch row {
	case 0: // k=1
		return 2, true
	case 1: // k=2
		return -2, true
	case 2: // k=3
		return -10, true
	case 3: // k=4
		return -26, true
	default: // k=5+
		// Double the previous value (recursive call is safe - mode check happens first)
		prevValue, _ := j.getFixedColumnValue(row - 1)
		return prevValue * 2, true
	}
}

// getDestinationIndexDifference calculates the difference between consecutive destination indexes
// for a given k value. Finds two consecutive odd numbers with that k value and returns
// the difference between their destination indexes.
// Returns (value, found) where found is true if we found actual consecutive numbers,
// false if we used the fallback pattern or hit search limit.
func (j *JumpsChapter) getDestinationIndexDifference(kValue int) (int, bool) {
	// For high k values, use the mathematical pattern to avoid overflow
	// The pattern for destination index differences is: 3 * 2^(k-1)
	if kValue >= 13 {
		// For k>=13, use pattern directly to avoid overflow issues
		// Pattern: 3 * 2^(k-1)
		powerOf2 := big.NewInt(1)
		powerOf2.Lsh(powerOf2, uint(kValue-1)) // 2^(k-1)
		result := new(big.Int).Mul(big.NewInt(3), powerOf2)
		if result.IsInt64() {
			return int(result.Int64()), false // false = used pattern, not found
		}
		// If it doesn't fit in int64, return 0 and indicate not found
		return 0, false
	}
	
	// For lower k values, try to find actual consecutive odd numbers
	// Find two consecutive odd numbers with this k value
	// Start from 1 and search
	var firstOdd, secondOdd int
	foundFirst := false
	
	// Increase search range for higher k values
	searchLimit := 10000
	if kValue > 5 {
		searchLimit = 50000 // Search more for higher k values
	}
	
	for n := 1; n < searchLimit; n += 2 { // Only check odd numbers
		if j.getFirstKValue(n) == kValue {
			if !foundFirst {
				firstOdd = n
				foundFirst = true
			} else {
				secondOdd = n
				break
			}
		}
	}
	
	if !foundFirst || secondOdd == 0 {
		// Search limit reached or not found - use fallback pattern
		// For k=1: difference is 6 (destinations: 5, 11, 17, ...)
		// For k>=2: 3 * 2^(k-1)
		if kValue == 1 {
			return 6, false // false = used pattern
		}
		// For k>=2: 3 * 2^(k-1)
		powerOf2 := big.NewInt(1)
		powerOf2.Lsh(powerOf2, uint(kValue-1)) // 2^(k-1)
		result := new(big.Int).Mul(big.NewInt(3), powerOf2)
		if result.IsInt64() {
			return int(result.Int64()), false // false = used pattern
		}
		return 0, false
	}
	
	// Calculate destination indexes using big.Int to avoid overflow
	firstDestBig := j.getCollatzNextBig(firstOdd)
	secondDestBig := j.getCollatzNextBig(secondOdd)
	
	// Calculate difference
	diff := new(big.Int).Sub(secondDestBig, firstDestBig)
	
	// Check if difference fits in int
	if !diff.IsInt64() {
		// Overflow - use pattern instead
		powerOf2 := big.NewInt(1)
		powerOf2.Lsh(powerOf2, uint(kValue-1)) // 2^(k-1)
		result := new(big.Int).Mul(big.NewInt(3), powerOf2)
		if result.IsInt64() {
			return int(result.Int64()), false // false = used pattern due to overflow
		}
		return 0, false
	}
	
	return int(diff.Int64()), true // true = found actual consecutive numbers
}

// Update updates the Jumps chapter
func (j *JumpsChapter) Update() error {
	j.escConsumed = false
	j.keyRepeatFrame++

	// Handle 'M' key to cycle through modes forward
	if inpututil.IsKeyJustPressed(ebiten.KeyM) && !ebiten.IsKeyPressed(ebiten.KeyShift) {
		j.displayMode = (j.displayMode + 1) % 5 // Cycle: 0 (Normal) -> 1 (Jump) -> 2 (Index) -> 3 (Incoming) -> 4 (SourceK) -> 0
	}
	
	// Handle Shift+M to cycle through modes backward
	if inpututil.IsKeyJustPressed(ebiten.KeyM) && ebiten.IsKeyPressed(ebiten.KeyShift) {
		j.displayMode = (j.displayMode - 1 + 5) % 5 // Cycle backward: 0 -> 4 -> 3 -> 2 -> 1 -> 0
	}

	// Handle arrow keys for navigation with proper key repeat
	initialDelay := 20 // Frames to wait before first repeat
	repeatDelay := 10  // Frames between repeats after initial delay

	// Helper function to move cursor
	moveCursor := func(deltaCol, deltaRow int) {
		j.selectedCol += deltaCol
		j.selectedRow += deltaRow

		// Clamp row to non-negative
		if j.selectedRow < 0 {
			j.selectedRow = 0
		}

		// Update scroll offsets to keep cursor visible
		if j.selectedCol < j.colScrollOffset {
			j.colScrollOffset = j.selectedCol
		} else if j.selectedCol >= j.colScrollOffset+j.visibleCols {
			j.colScrollOffset = j.selectedCol - j.visibleCols + 1
		}

		if j.selectedRow < j.rowScrollOffset {
			j.rowScrollOffset = j.selectedRow
		} else if j.selectedRow >= j.rowScrollOffset+j.visibleRows {
			j.rowScrollOffset = j.selectedRow - j.visibleRows + 1
		}
	}

	// Check each arrow key
	arrowKeys := []ebiten.Key{ebiten.KeyArrowLeft, ebiten.KeyArrowRight, ebiten.KeyArrowUp, ebiten.KeyArrowDown}
	for _, key := range arrowKeys {
		if inpututil.IsKeyJustPressed(key) {
			// First press - record the frame and allow immediate movement
			j.keyPressFrame[key] = j.keyRepeatFrame
			// Handle the key immediately
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
			// Key is held down - check if we should repeat
			if pressFrame, ok := j.keyPressFrame[key]; ok {
				framesSincePress := j.keyRepeatFrame - pressFrame
				if framesSincePress >= initialDelay {
					// After initial delay, repeat at regular intervals
					if (framesSincePress-initialDelay)%repeatDelay == 0 {
						// Handle the key repeat
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
			// Key is not pressed - remove from tracking
			delete(j.keyPressFrame, key)
		}
	}

	// Handle 'R' key to reset to column 1 (column index 0)
	if inpututil.IsKeyJustPressed(ebiten.KeyR) {
		j.selectedCol = 0 // Column index 0 = odd number 1
		j.selectedRow = 0
		j.colScrollOffset = 0
		j.rowScrollOffset = 0
	}

	// Handle Enter key to jump to incoming index
	if inpututil.IsKeyJustPressed(ebiten.KeyEnter) {
		currentOddNum := j.getOddNumberForCol(j.selectedCol)
		currentKValue := j.selectedRow + 1 // Row 0 = k=1
		
		// Get incoming source index
		var sourceIndex int
		if j.displayMode == 3 {
			// Incoming mode: cell shows the source index directly
			sourceIndex = j.getIncomingJumpSource(currentOddNum, currentKValue)
		} else if j.displayMode == 4 {
			// SourceK mode: need to calculate source index
			sourceIndex = j.getIncomingJumpSource(currentOddNum, currentKValue)
		}
		
		// If we found a source index, jump to it
		if sourceIndex != 0 {
			// Convert source index to column index
			// sourceIndex is an odd number, so colIndex = (sourceIndex - 1) / 2
			sourceColIndex := (sourceIndex - 1) / 2
			
			// Jump to that column
			j.selectedCol = sourceColIndex
			
			// Update scroll offset to keep cursor visible
			if j.selectedCol < j.colScrollOffset {
				j.colScrollOffset = j.selectedCol
			} else if j.selectedCol >= j.colScrollOffset+j.visibleCols {
				j.colScrollOffset = j.selectedCol - j.visibleCols + 1
			}
		}
	}

	return nil
}

// Draw draws the Jumps chapter
func (j *JumpsChapter) Draw(screen *ebiten.Image) {
	// Fill background
	screen.Fill(color.RGBA{15, 15, 15, 255})

	// Draw title
	titleText := "Jumps"
	titleBounds := text.BoundString(basicfont.Face7x13, titleText)
	titleX := (screenWidth - titleBounds.Dx()) / 2
	titleY := 20
	text.Draw(screen, titleText, basicfont.Face7x13, titleX, titleY, color.White)

	// Grid parameters
	cellWidth := 80
	cellHeight := 25
	colLabelWidth := 100
	rowLabelHeight := 30
	fixedColWidth := 100
	kLabelHeight := 15 // Height for outgoing k labels above column headers
	gridStartX := colLabelWidth + 10
	gridStartY := rowLabelHeight + 40 + kLabelHeight

	// Calculate visible range
	startCol := j.colScrollOffset
	endCol := startCol + j.visibleCols
	startRow := j.rowScrollOffset
	endRow := startRow + j.visibleRows

	// Draw outgoing k labels above column headers
	for col := startCol; col < endCol; col++ {
		oddNum := j.getOddNumberForCol(col)
		colX := gridStartX + (col-startCol)*cellWidth
		
		// Calculate outgoing k (first k value for this odd number)
		outgoingK := j.getFirstKValue(oddNum)
		kText := strconv.Itoa(outgoingK)
		kBounds := text.BoundString(basicfont.Face7x13, kText)
		kX := colX + (cellWidth-kBounds.Dx())/2
		kY := gridStartY - rowLabelHeight - 5
		text.Draw(screen, kText, basicfont.Face7x13, kX, kY, color.RGBA{150, 200, 255, 255}) // Light blue for k labels
	}

	// Draw column headers (odd numbers)
	for col := startCol; col < endCol; col++ {
		oddNum := j.getOddNumberForCol(col)
		colX := gridStartX + (col-startCol)*cellWidth

		headerText := strconv.Itoa(oddNum)
		headerBounds := text.BoundString(basicfont.Face7x13, headerText)
		headerX := colX + (cellWidth-headerBounds.Dx())/2
		headerY := gridStartY - 5
		
		// Color code index labels based on odd number mod 3 (only in Incoming and SourceK modes)
		var headerColor color.Color = color.White // Default white
		if j.displayMode == 3 || j.displayMode == 4 {
			oddMod3 := oddNum % 3
			if oddMod3 < 0 {
				oddMod3 += 3 // Handle negative numbers
			}
			switch oddMod3 {
			case 0:
				headerColor = color.RGBA{255, 100, 100, 255} // Red for 0 mod 3
			case 1:
				headerColor = color.RGBA{100, 255, 100, 255} // Green for 1 mod 3
			case 2:
				headerColor = color.RGBA{100, 100, 255, 255} // Blue for 2 mod 3
			}
		}
		text.Draw(screen, headerText, basicfont.Face7x13, headerX, headerY, headerColor)
	}

	// Draw color key on the far left (only in Incoming and SourceK modes)
	if j.displayMode == 3 || j.displayMode == 4 {
		colorKeyX := 10
		colorKeyY := gridStartY - rowLabelHeight - 5
		
		// Draw color key title
		keyTitle := "Mod 3:"
		text.Draw(screen, keyTitle, basicfont.Face7x13, colorKeyX, colorKeyY, color.White)
		
		// Draw color key items
		keyItemY := colorKeyY + 15
		keyBoxSize := 10
		keyTextX := colorKeyX + keyBoxSize + 5
		
		// 0 mod 3 - Red
		for dy := 0; dy < keyBoxSize; dy++ {
			for dx := 0; dx < keyBoxSize; dx++ {
				screen.Set(colorKeyX+dx, keyItemY+dy, color.RGBA{255, 100, 100, 255})
			}
		}
		text.Draw(screen, "0", basicfont.Face7x13, keyTextX, keyItemY+keyBoxSize-2, color.RGBA{255, 100, 100, 255})
		
		// 1 mod 3 - Green
		keyItemY += 15
		for dy := 0; dy < keyBoxSize; dy++ {
			for dx := 0; dx < keyBoxSize; dx++ {
				screen.Set(colorKeyX+dx, keyItemY+dy, color.RGBA{100, 255, 100, 255})
			}
		}
		text.Draw(screen, "1", basicfont.Face7x13, keyTextX, keyItemY+keyBoxSize-2, color.RGBA{100, 255, 100, 255})
		
		// 2 mod 3 - Blue
		keyItemY += 15
		for dy := 0; dy < keyBoxSize; dy++ {
			for dx := 0; dx < keyBoxSize; dx++ {
				screen.Set(colorKeyX+dx, keyItemY+dy, color.RGBA{100, 100, 255, 255})
			}
		}
		text.Draw(screen, "2", basicfont.Face7x13, keyTextX, keyItemY+keyBoxSize-2, color.RGBA{100, 100, 255, 255})
	}

	// Draw row labels (k values: 1, 2, 3, ...)
	for row := startRow; row < endRow; row++ {
		rowY := gridStartY + (row-startRow)*cellHeight
		kValue := row + 1 // Row 0 = k=1, row 1 = k=2, etc.
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

			oddNum := j.getOddNumberForCol(col)
			kValue := row + 1 // Row 0 = k=1

			// Draw cell background if selected
			isSelected := (col == j.selectedCol && row == j.selectedRow)
			if isSelected {
				// Draw selection highlight
				for dy := 0; dy < cellHeight-1; dy++ {
					for dx := 0; dx < cellWidth-1; dx++ {
						screen.Set(cellX+dx, cellY+dy, color.RGBA{100, 150, 255, 200})
					}
				}
			} else {
				// Draw cell border
				for dx := 0; dx < cellWidth-1; dx++ {
					screen.Set(cellX+dx, cellY, color.Gray{Y: 50})
					screen.Set(cellX+dx, cellY+cellHeight-2, color.Gray{Y: 50})
				}
				for dy := 0; dy < cellHeight-1; dy++ {
					screen.Set(cellX, cellY+dy, color.Gray{Y: 50})
					screen.Set(cellX+cellWidth-2, cellY+dy, color.Gray{Y: 50})
				}
			}

			// Calculate and draw cell content
			var cellText string
			var textColor color.Color = color.White // Default text color
			
			switch j.displayMode {
			case 0, 1, 2: // Normal, Jump, Index modes: only show in row matching first k
				firstK := j.getFirstKValue(oddNum)
				if firstK == kValue {
					// This is the row for this k value
					switch j.displayMode {
					case 0: // Normal mode: show k value
						cellText = strconv.Itoa(kValue)
					case 1: // Jump mode: show difference between current and next Collatz index
						jumpDiff := j.getJumpDifference(oddNum)
						cellText = strconv.Itoa(jumpDiff)
					case 2: // Index mode: show destination index
						nextOdd := j.getCollatzNext(oddNum)
						cellText = strconv.Itoa(nextOdd)
					}
				} else {
					// Not the row for this k value, show nothing
					cellText = ""
				}
			case 3: // Incoming mode: show source index that jumps to this destination with this k
				source := j.getIncomingJumpSource(oddNum, kValue)
				if source != 0 {
					cellText = strconv.Itoa(source)
					// Color code based on source mod 3
					sourceMod3 := source % 3
					if sourceMod3 < 0 {
						sourceMod3 += 3 // Handle negative numbers
					}
					switch sourceMod3 {
					case 0:
						textColor = color.RGBA{255, 100, 100, 255} // Red for 0 mod 3
					case 1:
						textColor = color.RGBA{100, 255, 100, 255} // Green for 1 mod 3
					case 2:
						textColor = color.RGBA{100, 100, 255, 255} // Blue for 2 mod 3
					}
				} else {
					// No incoming jump for this k value
					cellText = ""
				}
			case 4: // SourceK mode: show source k value (first k of incoming source)
				source := j.getIncomingJumpSource(oddNum, kValue)
				if source != 0 {
					sourceK := j.getFirstKValue(source)
					cellText = strconv.Itoa(sourceK)
					// Color code based on source mod 3
					sourceMod3 := source % 3
					if sourceMod3 < 0 {
						sourceMod3 += 3 // Handle negative numbers
					}
					switch sourceMod3 {
					case 0:
						textColor = color.RGBA{255, 100, 100, 255} // Red for 0 mod 3
					case 1:
						textColor = color.RGBA{100, 255, 100, 255} // Green for 1 mod 3
					case 2:
						textColor = color.RGBA{100, 100, 255, 255} // Blue for 2 mod 3
					}
				} else {
					// No incoming jump for this k value
					cellText = ""
				}
			}

			if cellText != "" {
				textBounds := text.BoundString(basicfont.Face7x13, cellText)
				textX := cellX + (cellWidth-textBounds.Dx())/2
				textY := cellY + cellHeight - 5
				text.Draw(screen, cellText, basicfont.Face7x13, textX, textY, textColor)
			}
		}
	}

	// Draw fixed column on the right
	fixedColX := screenWidth - fixedColWidth - 10
	fixedColHeaderY := gridStartY - 5
	headerText := "Jump"
	headerBounds := text.BoundString(basicfont.Face7x13, headerText)
	headerX := fixedColX + (fixedColWidth-headerBounds.Dx())/2
	text.Draw(screen, headerText, basicfont.Face7x13, headerX, fixedColHeaderY, color.RGBA{200, 150, 255, 255}) // Purple header

	// Draw fixed column cells
	for row := startRow; row < endRow; row++ {
		cellY := gridStartY + (row-startRow)*cellHeight

		// Draw fixed column background (different color)
		for dy := 0; dy < cellHeight-1; dy++ {
			for dx := 0; dx < fixedColWidth-1; dx++ {
				screen.Set(fixedColX+dx, cellY+dy, color.RGBA{40, 30, 50, 255}) // Dark purple background
			}
		}

		// Draw fixed column border
		for dx := 0; dx < fixedColWidth-1; dx++ {
			screen.Set(fixedColX+dx, cellY, color.RGBA{150, 100, 200, 255})
			screen.Set(fixedColX+dx, cellY+cellHeight-2, color.RGBA{150, 100, 200, 255})
		}
		for dy := 0; dy < cellHeight-1; dy++ {
			screen.Set(fixedColX, cellY+dy, color.RGBA{150, 100, 200, 255})
			screen.Set(fixedColX+fixedColWidth-2, cellY+dy, color.RGBA{150, 100, 200, 255})
		}

		// Draw fixed column value
		fixedValue, found := j.getFixedColumnValue(row)
		var valueText string
		if found {
			valueText = strconv.Itoa(fixedValue)
		} else {
			valueText = "x" // Show "x" if we used pattern/fallback or hit search limit
		}
		valueBounds := text.BoundString(basicfont.Face7x13, valueText)
		valueX := fixedColX + (fixedColWidth-valueBounds.Dx())/2
		valueY := cellY + cellHeight - 5
		text.Draw(screen, valueText, basicfont.Face7x13, valueX, valueY, color.RGBA{200, 150, 255, 255}) // Purple text
	}

	// Draw instructions
	modeText := "Normal"
	switch j.displayMode {
	case 0:
		modeText = "Normal"
	case 1:
		modeText = "Jump"
	case 2:
		modeText = "Index"
	case 3:
		modeText = "Incoming"
	case 4:
		modeText = "SourceK"
	}
	instructions := "Arrow Keys: Navigate | M: Cycle mode (" + modeText + ") | R: Reset to 1 | ESC: Return"
	instBounds := text.BoundString(basicfont.Face7x13, instructions)
	instX := (screenWidth - instBounds.Dx()) / 2
	instY := screenHeight - 30
	text.Draw(screen, instructions, basicfont.Face7x13, instX, instY, color.Gray{Y: 150})
}

// WasEscConsumed returns whether ESC was consumed by a dialog this frame
func (j *JumpsChapter) WasEscConsumed() bool {
	return j.escConsumed
}
