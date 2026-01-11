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

// TableChapter implements the Table chapter
type TableChapter struct {
	escConsumed bool

	// Selected cell position
	selectedCol int // Column index (odd number index: -3, -2, -1, 0, 1, 2, 3...)
	selectedRow int // Row index (0, 1, 2, ...)

	// Highlighted column (set with space bar)
	highlightedCol *int // Column index that is highlighted (nil if none)

	// Selection state (for rectangular selection with Shift+Arrow)
	selectionStartCol *int // Start column of selection (nil if no selection)
	selectionStartRow *int // Start row of selection
	selectionEndCol   int   // End column of selection (current selectedCol)
	selectionEndRow   int   // End row of selection (current selectedRow)

	// Scroll offsets
	colScrollOffset int // Horizontal scroll offset (column index)
	rowScrollOffset int // Vertical scroll offset (row index)

	// Visible area
	visibleCols int
	visibleRows int

	// Key repeat
	keyRepeatFrame int
	keyPressFrame   map[ebiten.Key]int // Track when each key was first pressed
}

// NewTableChapter creates a new Table chapter
func NewTableChapter() *TableChapter {
	// Calculate visible columns and rows based on screen size
	cellWidth := 80
	cellHeight := 25
	colLabelWidth := 100
	rowLabelHeight := 30

	availableWidth := screenWidth - colLabelWidth - 20
	availableHeight := screenHeight - rowLabelHeight - 60 // Title + instructions

	visibleCols := availableWidth / cellWidth
	visibleRows := availableHeight / cellHeight

	if visibleCols < 1 {
		visibleCols = 1
	}
	if visibleRows < 1 {
		visibleRows = 1
	}

	return &TableChapter{
		selectedCol:     0, // Start at column 0 (odd number 1)
		selectedRow:     0, // Start at row 0
		colScrollOffset: 0,
		rowScrollOffset: 0,
		visibleCols:    visibleCols,
		visibleRows:    visibleRows,
		keyPressFrame:  make(map[ebiten.Key]int),
	}
}

// getOddNumberForCol returns the odd number for a given column index
// Column index 0 = odd number 1
// Column index 1 = odd number 3
// Column index -1 = odd number -1
// Column index -2 = odd number -3
// Pattern: odd = 2*colIndex + 1 (works for both positive and negative)
func (t *TableChapter) getOddNumberForCol(colIndex int) int {
	return 2*colIndex + 1
}

// getColIndexForOddNumber returns the column index for a given odd number
func (t *TableChapter) getColIndexForOddNumber(oddNum int) int {
	// Formula: colIndex = (oddNum - 1) / 2
	// Works for both positive and negative odd numbers
	return (oddNum - 1) / 2
}

// calculateKValue calculates the k value for an odd number at a given index
// Index 0: first k value (after 1 Collatz step)
// Index 1: second k value (after 2 Collatz steps)
// etc.
func (t *TableChapter) calculateKValue(oddNum int, index int) int {
	if index < 0 {
		return 0
	}

	result := big.NewInt(int64(oddNum))
	two := big.NewInt(2)

	// Perform index+1 Collatz steps
	for step := 0; step <= index; step++ {
		// Collatz step: result * 3 + 1
		result.Mul(result, big.NewInt(3))
		result.Add(result, big.NewInt(1))

		// Count how many times the result is divisible by 2 (k for this step)
		k := findLargestPowerOf2Big(result)

		// Divide by 2^k
		powerOf2 := new(big.Int).Exp(two, big.NewInt(int64(k)), nil)
		result.Div(result, powerOf2)

		// If this is the step we want, return the k value
		if step == index {
			return k
		}
	}

	return 0
}

// calculateFrequency calculates the frequency for a column's selected row range
// Frequency is the product of 2^k for all k values in the selected rows
func (t *TableChapter) calculateFrequency(colIndex int, startRow, endRow int) *big.Int {
	if startRow < 0 || endRow < 0 {
		return big.NewInt(1)
	}

	// Ensure startRow <= endRow
	if startRow > endRow {
		startRow, endRow = endRow, startRow
	}

	oddNum := t.getOddNumberForCol(colIndex)
	frequency := big.NewInt(1)

	// Multiply by 2^k for each row in the selection
	for row := startRow; row <= endRow; row++ {
		kValue := t.calculateKValue(oddNum, row)
		if kValue > 0 {
			// Calculate 2^k using big.Int
			powerOf2 := new(big.Int).Lsh(big.NewInt(1), uint(kValue)) // 1 << k
			frequency.Mul(frequency, powerOf2)
		}
		// If k is 0, we multiply by 1 (no change), so we can skip it
	}

	return frequency
}

// Update updates the Table chapter
func (t *TableChapter) Update() error {
	t.escConsumed = false
	t.keyRepeatFrame++

	// Handle ESC - don't consume it, let main menu handle it
	// (ESC is not consumed by this chapter)

	// Handle space bar to highlight/unhighlight current column
	if inpututil.IsKeyJustPressed(ebiten.KeySpace) {
		if t.highlightedCol != nil && *t.highlightedCol == t.selectedCol {
			// Unhighlight if clicking the same column
			t.highlightedCol = nil
		} else {
			// Highlight current column
			col := t.selectedCol
			t.highlightedCol = &col
		}
	}

	// Check if Shift is pressed
	shiftPressed := ebiten.IsKeyPressed(ebiten.KeyShift)

	// Handle arrow keys for navigation with proper key repeat
	initialDelay := 20  // Frames to wait before first repeat
	repeatDelay := 10  // Frames between repeats after initial delay

	// Helper function to move cursor and update selection
	moveCursor := func(deltaCol, deltaRow int) {
		// If Shift is pressed, extend selection; otherwise, clear selection
		if shiftPressed {
			// Extend selection
			if t.selectionStartCol == nil {
				// Start new selection at current position (before moving)
				startCol := t.selectedCol
				startRow := t.selectedRow
				t.selectionStartCol = &startCol
				t.selectionStartRow = &startRow
			}
		} else {
			// Clear selection when not holding Shift
			t.selectionStartCol = nil
			t.selectionStartRow = nil
		}

		// Move cursor
		t.selectedCol += deltaCol
		t.selectedRow += deltaRow

		// Update selection end if in selection mode
		if shiftPressed && t.selectionStartCol != nil {
			t.selectionEndCol = t.selectedCol
			t.selectionEndRow = t.selectedRow
		}

		// Clamp row to non-negative
		if t.selectedRow < 0 {
			t.selectedRow = 0
		}

		// Update scroll offsets to keep cursor visible
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

	// Check each arrow key
	arrowKeys := []ebiten.Key{ebiten.KeyArrowLeft, ebiten.KeyArrowRight, ebiten.KeyArrowUp, ebiten.KeyArrowDown}
	for _, key := range arrowKeys {
		if inpututil.IsKeyJustPressed(key) {
			// First press - record the frame and allow immediate movement
			t.keyPressFrame[key] = t.keyRepeatFrame
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
			if pressFrame, ok := t.keyPressFrame[key]; ok {
				framesSincePress := t.keyRepeatFrame - pressFrame
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
			delete(t.keyPressFrame, key)
		}
	}

	// Clear selection when Shift is released
	if !shiftPressed && t.selectionStartCol != nil {
		// Keep selection until explicitly cleared or new selection started
		// (Selection persists even after releasing Shift)
	}

	// Handle 'R' key to reset to column 1 (column index 0)
	if inpututil.IsKeyJustPressed(ebiten.KeyR) {
		t.selectedCol = 0 // Column index 0 = odd number 1
		t.selectedRow = 0
		t.colScrollOffset = 0
		t.rowScrollOffset = 0
		t.selectionStartCol = nil
		t.selectionStartRow = nil
		t.highlightedCol = nil
	}

	return nil
}

// Draw draws the Table chapter
func (t *TableChapter) Draw(screen *ebiten.Image) {
	// Fill background
	screen.Fill(color.RGBA{15, 15, 15, 255})

	// Draw title
	titleText := "Table"
	titleBounds := text.BoundString(basicfont.Face7x13, titleText)
	titleX := (screenWidth - titleBounds.Dx()) / 2
	titleY := 20
	text.Draw(screen, titleText, basicfont.Face7x13, titleX, titleY, color.White)

	// Grid parameters
	cellWidth := 80
	cellHeight := 25
	colLabelWidth := 100
	rowLabelHeight := 30
	frequencyHeight := 15 // Space for frequency display above headers
	gridStartX := colLabelWidth + 10
	gridStartY := rowLabelHeight + 40 + frequencyHeight

	// Calculate visible range
	startCol := t.colScrollOffset
	endCol := startCol + t.visibleCols
	startRow := t.rowScrollOffset
	endRow := startRow + t.visibleRows

	// Calculate selection bounds if there's a selection
	var selStartCol, selEndCol, selStartRow, selEndRow int
	hasSelection := false
	if t.selectionStartCol != nil && t.selectionStartRow != nil {
		hasSelection = true
		selStartCol = *t.selectionStartCol
		selEndCol = t.selectionEndCol
		selStartRow = *t.selectionStartRow
		selEndRow = t.selectionEndRow
		// Ensure start <= end
		if selStartCol > selEndCol {
			selStartCol, selEndCol = selEndCol, selStartCol
		}
		if selStartRow > selEndRow {
			selStartRow, selEndRow = selEndRow, selStartRow
		}
	}

	// Draw column headers (odd numbers) and frequencies
	for col := startCol; col < endCol; col++ {
		oddNum := t.getOddNumberForCol(col)
		colX := gridStartX + (col-startCol)*cellWidth
		
		// Check if this column is highlighted
		isHighlighted := t.highlightedCol != nil && *t.highlightedCol == col
		
		// Draw header background if highlighted
		if isHighlighted {
			for dx := 0; dx < cellWidth-1; dx++ {
				for dy := 0; dy < rowLabelHeight-5; dy++ {
					screen.Set(colX+dx, gridStartY-5+dy, color.RGBA{255, 200, 100, 200}) // Orange highlight
				}
			}
		}
		
		headerText := strconv.Itoa(oddNum)
		headerBounds := text.BoundString(basicfont.Face7x13, headerText)
		headerX := colX + (cellWidth-headerBounds.Dx())/2
		headerY := gridStartY - 5
		text.Draw(screen, headerText, basicfont.Face7x13, headerX, headerY, color.White)

		// Draw frequency above header if this column is in the selection
		if hasSelection && col >= selStartCol && col <= selEndCol {
			frequency := t.calculateFrequency(col, selStartRow, selEndRow)
			freqText := frequency.String()
			freqBounds := text.BoundString(basicfont.Face7x13, freqText)
			freqX := colX + (cellWidth-freqBounds.Dx())/2
			freqY := gridStartY - rowLabelHeight - frequencyHeight
			text.Draw(screen, freqText, basicfont.Face7x13, freqX, freqY, color.RGBA{100, 255, 100, 255}) // Green for frequency
		}
	}

	// Draw row labels (indices)
	for row := startRow; row < endRow; row++ {
		rowY := gridStartY + (row-startRow)*cellHeight
		labelText := strconv.Itoa(row)
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

			oddNum := t.getOddNumberForCol(col)
			kValue := t.calculateKValue(oddNum, row)

			// Check if this column is highlighted
			isHighlighted := t.highlightedCol != nil && *t.highlightedCol == col
			
			// Check if this cell is in the selection
			isInSelection := false
			if hasSelection {
				isInSelection = (col >= selStartCol && col <= selEndCol && row >= selStartRow && row <= selEndRow)
			}
			
			// Draw cell background if selected
			isSelected := (col == t.selectedCol && row == t.selectedRow)
			if isSelected {
				// Draw selection highlight
				for dy := 0; dy < cellHeight-1; dy++ {
					for dx := 0; dx < cellWidth-1; dx++ {
						screen.Set(cellX+dx, cellY+dy, color.RGBA{100, 150, 255, 200})
					}
				}
			} else if isInSelection {
				// Draw selection region highlight (lighter blue)
				for dy := 0; dy < cellHeight-1; dy++ {
					for dx := 0; dx < cellWidth-1; dx++ {
						screen.Set(cellX+dx, cellY+dy, color.RGBA{100, 150, 255, 100})
					}
				}
			} else if isHighlighted {
				// Draw highlight for highlighted column (lighter orange)
				for dy := 0; dy < cellHeight-1; dy++ {
					for dx := 0; dx < cellWidth-1; dx++ {
						screen.Set(cellX+dx, cellY+dy, color.RGBA{255, 200, 100, 150})
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

			// Draw k value
			kText := strconv.Itoa(kValue)
			kBounds := text.BoundString(basicfont.Face7x13, kText)
			kX := cellX + (cellWidth-kBounds.Dx())/2
			kY := cellY + cellHeight - 5
			text.Draw(screen, kText, basicfont.Face7x13, kX, kY, color.White)
		}
	}
	
	// Draw offset count if a column is highlighted
	if t.highlightedCol != nil {
		offset := t.selectedCol - *t.highlightedCol
		offsetText := "Offset: " + strconv.Itoa(offset)
		offsetBounds := text.BoundString(basicfont.Face7x13, offsetText)
		offsetX := (screenWidth - offsetBounds.Dx()) / 2
		offsetY := screenHeight - 50
		text.Draw(screen, offsetText, basicfont.Face7x13, offsetX, offsetY, color.RGBA{255, 200, 100, 255})
	}

	// Draw instructions
	instructions := "Arrow Keys: Navigate | Shift+Arrows: Select region | Space: Highlight column | R: Reset to 1 | ESC: Return"
	instBounds := text.BoundString(basicfont.Face7x13, instructions)
	instX := (screenWidth - instBounds.Dx()) / 2
	instY := screenHeight - 30
	text.Draw(screen, instructions, basicfont.Face7x13, instX, instY, color.Gray{Y: 150})
}

// WasEscConsumed returns whether ESC was consumed by a dialog this frame
func (t *TableChapter) WasEscConsumed() bool {
	return t.escConsumed
}
