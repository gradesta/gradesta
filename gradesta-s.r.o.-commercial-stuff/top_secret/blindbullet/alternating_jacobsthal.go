package main

import (
	"fmt"
	"image/color"
	"math/big"
	"strconv"

	"github.com/hajimehoshi/ebiten/v2"
	"github.com/hajimehoshi/ebiten/v2/ebitenutil"
	"github.com/hajimehoshi/ebiten/v2/inpututil"
	"github.com/hajimehoshi/ebiten/v2/text"
	"golang.org/x/image/font/basicfont"
)

// AlternatingJacobsthalChapter implements The Alternating Jacobsthal sequence chapter
type AlternatingJacobsthalChapter struct {
	// Currently selected cell (0-indexed, right to left, top to bottom)
	// Cell 0 is top-left, cell 1 is top-right, cell 2 is second row left, etc.
	selectedCell int
	
	// Scroll offset for displaying rows (how many rows scrolled)
	scrollOffset int
	
	// Generated sequence parameters from Enter key press
	// Instead of storing the sequence, we store the parameters to generate it infinitely
	sequenceCellNum      int  // Cell number used to generate the sequence
	sequenceCellValue    int  // Original cell value used to generate the sequence
	sequenceOffset       int  // Offset for indexing into sequence with Jacobsthal numbers
	sequenceScrollOffset int  // Scroll offset for displaying the sequence (i value to start from)
	
	// Whether we're using the generated sequence to refill the table
	usingGeneratedSequence bool
	
	// History of cell numbers and k values when Enter was pressed
	history []struct {
		cellNum int
		kValue  int
	}
	
	// Offset input dialogs
	showOffsetDialog bool
	offsetInputBuffer string
	showScrollOffsetDialog bool
	scrollOffsetInputBuffer string
	
	// Help dialog
	showHelp bool
	helpScrollOffset int
	
	escConsumed bool // Whether ESC was consumed by a dialog this frame
}

// NewAlternatingJacobsthalChapter creates a new Alternating Jacobsthal chapter
func NewAlternatingJacobsthalChapter() *AlternatingJacobsthalChapter {
	return &AlternatingJacobsthalChapter{
		selectedCell:           0,
		scrollOffset:           0,
		sequenceCellNum:        -1,
		sequenceCellValue:      0,
		sequenceOffset:         0,
		sequenceScrollOffset:   -10, // Start showing from i = -10
		usingGeneratedSequence: false,
		escConsumed:            false,
	}
}

// jacobsthal calculates the n-th Jacobsthal number
// J(0) = 0, J(1) = 1, J(n) = J(n-1) + 2*J(n-2)
func jacobsthal(n int) int {
	if n < 0 {
		return 0 // Handle negative indices gracefully
	}
	if n == 0 {
		return 0
	}
	if n == 1 {
		return 1
	}
	
	// Use dynamic programming to calculate
	j := make([]int, n+1)
	j[0] = 0
	j[1] = 1
	for i := 2; i <= n; i++ {
		j[i] = j[i-1] + 2*j[i-2]
	}
	return j[n]
}

// calculateNextKValue calculates the "next k value" for a number
// The number of Collatz steps depends on history length:
// - 0 items in history: 1 step (first k value)
// - 1 item in history: 2 steps (second k value)
// - 2 items in history: 3 steps (third k value)
// etc.
func (a *AlternatingJacobsthalChapter) calculateNextKValue(n int) int {
	// Number of steps = history length + 1
	numSteps := len(a.history) + 1
	
	result := big.NewInt(int64(n))
	two := big.NewInt(2)
	
	// Perform numSteps Collatz steps
	for step := 0; step < numSteps; step++ {
		// Collatz step: result * 3 + 1
		result.Mul(result, big.NewInt(3))
		result.Add(result, big.NewInt(1))
		
		// Count how many times the result is divisible by 2 (k for this step)
		k := findLargestPowerOf2Big(result)
		
		// Divide by 2^k
		powerOf2 := new(big.Int).Exp(two, big.NewInt(int64(k)), nil)
		result.Div(result, powerOf2)
		
		// If this is the last step, return the k value
		if step == numSteps-1 {
			return k
		}
	}
	
	return 0 // Should never reach here
}

// getOriginalCellValue returns the original alternating Jacobsthal sequence value for a cell
func (a *AlternatingJacobsthalChapter) getOriginalCellValue(cellNum int) int {
	// Handle negative cell numbers gracefully
	if cellNum < 0 {
		return 0
	}
	
	// Right column has even cell numbers (0, 2, 4, 6, ...)
	// Left column has odd cell numbers (1, 3, 5, 7, ...)
	row := cellNum / 2
	isRight := cellNum%2 == 0 // Even = right, odd = left
	
	// Pattern from examples:
	// Row 0: right=0=J(0), left=-1=-J(2)
	// Row 1: right=1=J(1), left=-3=-J(3)
	// Row 2: right=5=J(4), left=-11=-J(5)
	// Row 3: right=21=J(6), left=-43=-J(7)
	// Row 4: right=85=J(8), left=-171=-J(9)
	
	if isRight {
		// Right column (positive) - even cell numbers
		if row == 0 {
			return jacobsthal(0) // 0
		} else if row == 1 {
			return jacobsthal(1) // 1
		} else {
			// J(2*row) for row >= 2
			// Row 2: J(4) = 5 ✓
			// Row 3: J(6) = 21
			return jacobsthal(2 * row)
		}
	} else {
		// Left column (negative) - odd cell numbers
		// Row 0: -J(2) = -1
		// Row 1: -J(3) = -3
		// Row 2: -J(5) = -11
		// Pattern: -J(2*row + 2) for row 0, -J(2*row + 1) for row >= 1
		if row == 0 {
			return -jacobsthal(2) // -1
		} else {
			return -jacobsthal(2*row + 1) // -J(3), -J(5), -J(7), ...
		}
	}
}

// getSequenceValue calculates the infinite sequence value for a given index i
// Formula: jacobsthal_number + i * 2^cell_number (changed from i * 2 * 2^cell_number)
func (a *AlternatingJacobsthalChapter) getSequenceValue(i int) int {
	if !a.usingGeneratedSequence || a.sequenceCellNum < 0 {
		return 0
	}
	// Calculate: sequenceCellValue + i * 2^sequenceCellNum
	powerOf2 := 1 << uint(a.sequenceCellNum) // 2^sequenceCellNum
	return a.sequenceCellValue + i*powerOf2
}

// getCellValue returns the value for a given cell number
// If using generated sequence: get jacobsthal value, then apply offset, then use as index into generated sequence
func (a *AlternatingJacobsthalChapter) getCellValue(cellNum int) int {
	// If using generated sequence, get value from sequence using original alternating sequence as index
	if a.usingGeneratedSequence {
		// Get i from the original alternating Jacobsthal sequence: i = jacobsthal_alternating_sequence[cell_number]
		originalValue := a.getOriginalCellValue(cellNum)
		
		// Apply offset to the Jacobsthal value: i = originalValue + offset
		indexValue := originalValue + a.sequenceOffset
		
		// Use this value as i to calculate the sequence value (infinite sequence, calculated on demand)
		return a.getSequenceValue(indexValue)
	}
	
	// Default behavior: return original Jacobsthal sequence values
	return a.getOriginalCellValue(cellNum)
}

// getCellRow returns the row index (0-based) for a given cell number
func (a *AlternatingJacobsthalChapter) getCellRow(cellNum int) int {
	return cellNum / 2
}

// getCellCol returns whether the cell is in the right column (true) or left column (false)
func (a *AlternatingJacobsthalChapter) getCellCol(cellNum int) bool {
	return cellNum%2 == 0 // Even = right, odd = left
}

// WasEscConsumed returns whether ESC was consumed by a dialog this frame
func (a *AlternatingJacobsthalChapter) WasEscConsumed() bool {
	return a.escConsumed
}

// Update updates the Alternating Jacobsthal chapter
func (a *AlternatingJacobsthalChapter) Update() error {
	a.escConsumed = false
	
	// Handle 'h' key to toggle help dialog
	if inpututil.IsKeyJustPressed(ebiten.KeyH) {
		a.showHelp = !a.showHelp
		if a.showHelp {
			a.helpScrollOffset = 0
		}
		return nil
	}
	
	// Handle help dialog scrolling
	if a.showHelp {
		if inpututil.IsKeyJustPressed(ebiten.KeyArrowDown) || inpututil.IsKeyJustPressed(ebiten.KeyS) {
			a.helpScrollOffset += 15
		}
		if inpututil.IsKeyJustPressed(ebiten.KeyArrowUp) || inpututil.IsKeyJustPressed(ebiten.KeyW) {
			a.helpScrollOffset -= 15
			if a.helpScrollOffset < 0 {
				a.helpScrollOffset = 0
			}
		}
		// Close help with Escape or 'h' again
		if inpututil.IsKeyJustPressed(ebiten.KeyEscape) || inpututil.IsKeyJustPressed(ebiten.KeyH) {
			a.showHelp = false
			a.escConsumed = true
		}
		// Don't process other keys when help is open
		return nil
	}
	
	// Handle offset input dialogs first (consumes all input when open)
	if a.showOffsetDialog {
		// Handle ESC to cancel
		if inpututil.IsKeyJustPressed(ebiten.KeyEscape) {
			a.showOffsetDialog = false
			a.offsetInputBuffer = ""
			a.escConsumed = true
			return nil
		}
		
		// Handle Enter to confirm
		if inpututil.IsKeyJustPressed(ebiten.KeyEnter) {
			// Parse the input as an integer
			if len(a.offsetInputBuffer) > 0 {
				newOffset, err := strconv.Atoi(a.offsetInputBuffer)
				if err == nil {
					a.sequenceOffset = newOffset
				}
			}
			a.showOffsetDialog = false
			a.offsetInputBuffer = ""
			return nil
		}
		
		// Handle Backspace
		if inpututil.IsKeyJustPressed(ebiten.KeyBackspace) {
			if len(a.offsetInputBuffer) > 0 {
				a.offsetInputBuffer = a.offsetInputBuffer[:len(a.offsetInputBuffer)-1]
			}
		}
		
		// Handle character input (works with any keyboard layout)
		chars := ebiten.AppendInputChars(nil)
		for _, char := range chars {
			charStr := string(char)
			// Allow digits 0-9
			if char >= '0' && char <= '9' {
				a.offsetInputBuffer += charStr
			}
			// Allow minus sign only at the start
			if char == '-' && len(a.offsetInputBuffer) == 0 {
				a.offsetInputBuffer = "-"
			}
		}
		
		return nil
	}
	
	// Handle scroll offset input dialog
	if a.showScrollOffsetDialog {
		// Handle ESC to cancel
		if inpututil.IsKeyJustPressed(ebiten.KeyEscape) {
			a.showScrollOffsetDialog = false
			a.scrollOffsetInputBuffer = ""
			a.escConsumed = true
			return nil
		}
		
		// Handle Enter to confirm
		if inpututil.IsKeyJustPressed(ebiten.KeyEnter) {
			// Parse the input as a sequence value (not an index)
			if len(a.scrollOffsetInputBuffer) > 0 {
				targetValue, err := strconv.Atoi(a.scrollOffsetInputBuffer)
				if err == nil {
					// Find the index that produces this sequence value
					// Formula: sequenceValue = sequenceCellValue + i * 2^sequenceCellNum
					// So: i = (targetValue - sequenceCellValue) / 2^sequenceCellNum
					powerOf2 := 1 << uint(a.sequenceCellNum)
					index := (targetValue - a.sequenceCellValue) / powerOf2
					
					// Round to nearest even number (since we use even indices only)
					if index%2 != 0 {
						if index < 0 {
							index = index - 1
						} else {
							index = index + 1
						}
					}
					
					a.sequenceScrollOffset = index
				}
			}
			a.showScrollOffsetDialog = false
			a.scrollOffsetInputBuffer = ""
			return nil
		}
		
		// Handle Backspace
		if inpututil.IsKeyJustPressed(ebiten.KeyBackspace) {
			if len(a.scrollOffsetInputBuffer) > 0 {
				a.scrollOffsetInputBuffer = a.scrollOffsetInputBuffer[:len(a.scrollOffsetInputBuffer)-1]
			}
		}
		
		// Handle character input (works with any keyboard layout)
		chars := ebiten.AppendInputChars(nil)
		for _, char := range chars {
			charStr := string(char)
			// Allow digits 0-9
			if char >= '0' && char <= '9' {
				a.scrollOffsetInputBuffer += charStr
			}
			// Allow minus sign only at the start
			if char == '-' && len(a.scrollOffsetInputBuffer) == 0 {
				a.scrollOffsetInputBuffer = "-"
			}
		}
		
		return nil
	}
	
	// Handle 'o' key to open offset input dialog
	if inpututil.IsKeyJustPressed(ebiten.KeyO) {
		if a.usingGeneratedSequence {
			a.showOffsetDialog = true
			a.offsetInputBuffer = strconv.Itoa(a.sequenceOffset)
			return nil
		}
	}
	
	// Handle 'p' key to open scroll offset input dialog
	if inpututil.IsKeyJustPressed(ebiten.KeyP) {
		if a.usingGeneratedSequence {
			a.showScrollOffsetDialog = true
			a.scrollOffsetInputBuffer = strconv.Itoa(a.sequenceScrollOffset)
			return nil
		}
	}
	
	// Check if shift is pressed for offset adjustment
	shiftPressed := ebiten.IsKeyPressed(ebiten.KeyShiftLeft) || ebiten.IsKeyPressed(ebiten.KeyShiftRight)
	
	// Handle Shift+Left/Right to adjust sequence offset (check this first)
	if shiftPressed {
		if inpututil.IsKeyJustPressed(ebiten.KeyArrowLeft) {
			a.sequenceOffset--
		}
		if inpututil.IsKeyJustPressed(ebiten.KeyArrowRight) || inpututil.IsKeyJustPressed(ebiten.KeyD) {
			a.sequenceOffset++
		}
	} else {
		// Handle arrow key navigation (only if shift is not pressed)
		// Right arrow: move to right column (or next row if already on right)
		if inpututil.IsKeyJustPressed(ebiten.KeyArrowRight) || inpututil.IsKeyJustPressed(ebiten.KeyD) {
			currentRow := a.getCellRow(a.selectedCell)
			isRight := a.getCellCol(a.selectedCell)
			
			if !isRight {
				// Move to right column of same row (even cell number)
				a.selectedCell = currentRow * 2
			} else {
				// Move to right column of next row
				a.selectedCell = (currentRow + 1) * 2
			}
		}
		
		// Left arrow: move to left column (or previous row if already on left)
		if inpututil.IsKeyJustPressed(ebiten.KeyArrowLeft) {
			currentRow := a.getCellRow(a.selectedCell)
			isRight := a.getCellCol(a.selectedCell)
			
			if isRight {
				// Move to left column of same row (odd cell number)
				a.selectedCell = currentRow*2 + 1
			} else if currentRow > 0 {
				// Move to left column of previous row
				a.selectedCell = (currentRow-1)*2 + 1
			}
		}
	}
	
	// Down arrow: move down (to next row, same column)
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowDown) || inpututil.IsKeyJustPressed(ebiten.KeyS) {
		currentRow := a.getCellRow(a.selectedCell)
		isRight := a.getCellCol(a.selectedCell)
		if isRight {
			a.selectedCell = (currentRow + 1) * 2
		} else {
			a.selectedCell = (currentRow + 1)*2 + 1
		}
	}
	
	// Up arrow: move up (to previous row, same column)
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowUp) || inpututil.IsKeyJustPressed(ebiten.KeyW) {
		currentRow := a.getCellRow(a.selectedCell)
		if currentRow > 0 {
			isRight := a.getCellCol(a.selectedCell)
			if isRight {
				a.selectedCell = (currentRow - 1) * 2
			} else {
				a.selectedCell = (currentRow-1)*2 + 1
			}
		}
	}
	
	// Handle Enter key to generate sequence from selected cell
	if inpututil.IsKeyJustPressed(ebiten.KeyEnter) || inpututil.IsKeyJustPressed(ebiten.KeySpace) {
		cellNum := a.selectedCell
		// Use original cell value (not the refilled value) to generate the sequence
		cellValue := a.getOriginalCellValue(cellNum)
		
		// Calculate k value for this cell
		kValue := a.calculateNextKValue(cellValue)
		
		// Add to history
		a.history = append(a.history, struct {
			cellNum int
			kValue  int
		}{cellNum: cellNum, kValue: kValue})
		
		// Store parameters to generate infinite sequence on demand
		// Formula: jacobsthal_number + i * 2^cell_number (changed from 2 * 2^cell_number)
		a.sequenceCellNum = cellNum
		a.sequenceCellValue = cellValue
		a.sequenceOffset = 0
		a.sequenceScrollOffset = -4 // Reset scroll to start at -4 (even numbers: -4, -2, 0, 2, ...)
		a.usingGeneratedSequence = true
	}
	
	// Handle 'l' key to scroll sequence right (increase i by 2, keeping it even)
	if inpututil.IsKeyJustPressed(ebiten.KeyL) {
		a.sequenceScrollOffset += 2
	}
	
	// Handle 'k' key to scroll sequence left (decrease i by 2, keeping it even)
	if inpututil.IsKeyJustPressed(ebiten.KeyK) {
		a.sequenceScrollOffset -= 2
	}
	
	// Check if offset produces matching k values for cells 1, 2, 3
	// Returns true if cell 1's k = 1, cell 2's k = 2, cell 3's k = 3
	checkOffsetMatch := func(offset int) bool {
		if !a.usingGeneratedSequence {
			return false
		}
		// Temporarily set offset to test
		oldOffset := a.sequenceOffset
		a.sequenceOffset = offset
		
		// Check cells 1, 2, 3
		// Cell 1: left column, row 0
		// Cell 2: right column, row 1
		// Cell 3: left column, row 1
		cell1Value := a.getCellValue(1)
		cell2Value := a.getCellValue(2)
		cell3Value := a.getCellValue(3)
		
		k1 := a.calculateNextKValue(cell1Value)
		k2 := a.calculateNextKValue(cell2Value)
		k3 := a.calculateNextKValue(cell3Value)
		
		// Restore offset
		a.sequenceOffset = oldOffset
		
		// Check if k values match cell numbers (1, 2, 3)
		return k1 == 1 && k2 == 2 && k3 == 3
	}
	
	// Handle 'a' key to find offset to the left (decrease) where cells 1,2,3 match
	if inpututil.IsKeyJustPressed(ebiten.KeyA) && !shiftPressed {
		if a.usingGeneratedSequence {
			// Search left (decrease offset) until we find a match
			startOffset := a.sequenceOffset
			for testOffset := startOffset - 1; testOffset >= startOffset - 1000; testOffset-- {
				if checkOffsetMatch(testOffset) {
					a.sequenceOffset = testOffset
					break
				}
			}
		}
	}
	
	// Handle 's' key to find offset to the right (increase) where cells 1,2,3 match
	if inpututil.IsKeyJustPressed(ebiten.KeyS) {
		if a.usingGeneratedSequence {
			// Search right (increase offset) until we find a match
			startOffset := a.sequenceOffset
			for testOffset := startOffset + 1; testOffset <= startOffset + 1000; testOffset++ {
				if checkOffsetMatch(testOffset) {
					a.sequenceOffset = testOffset
					break
				}
			}
		}
	}
	
	// Handle 'r' key to reset (clear generated sequence and history)
	if inpututil.IsKeyJustPressed(ebiten.KeyR) {
		a.usingGeneratedSequence = false
		a.sequenceCellNum = -1
		a.sequenceCellValue = 0
		a.sequenceOffset = 0
		a.sequenceScrollOffset = -4 // Reset to -4 (even numbers)
		a.history = nil // Clear history
	}
	
	// Update scroll offset to keep selected cell visible
	selectedRow := a.getCellRow(a.selectedCell)
	visibleRows := 15 // Number of rows visible on screen
	if selectedRow < a.scrollOffset {
		a.scrollOffset = selectedRow
	} else if selectedRow >= a.scrollOffset+visibleRows {
		a.scrollOffset = selectedRow - visibleRows + 1
	}
	
	return nil
}

// Draw draws the Alternating Jacobsthal chapter
func (a *AlternatingJacobsthalChapter) Draw(screen *ebiten.Image) {
	// Fill background
	screen.Fill(color.RGBA{20, 20, 30, 255})
	
	// Draw title
	titleText := "The Alternating Jacobsthal sequence"
	titleBounds := text.BoundString(basicfont.Face7x13, titleText)
	titleX := (screenWidth - titleBounds.Dx()) / 2
	titleY := 20
	text.Draw(screen, titleText, basicfont.Face7x13, titleX, titleY, color.White)
	
	// Table parameters
	cellWidth := 150
	cellHeight := 30
	tableStartX := screenWidth/2 - cellWidth
	tableStartY := 60
	visibleRows := 15
	
	// Draw visible rows
	for i := 0; i < visibleRows; i++ {
		row := a.scrollOffset + i
		rowY := tableStartY + i*cellHeight
		
		// Cell numbering: right to left, top to bottom
		// Row 0: right cell = 0 (even), left cell = 1 (odd)
		// Row 1: right cell = 2 (even), left cell = 3 (odd)
		// Row 2: right cell = 4 (even), left cell = 5 (odd)
		// Displayed as: left, right (so we see "1, 0 / 3, 2 / 5, 4")
		rightCellNum := row * 2      // Even numbers: right column
		leftCellNum := row*2 + 1    // Odd numbers: left column
		
		// Draw left cell (negative) - displayed on left, but numbered second
		leftValue := a.getCellValue(leftCellNum)
		leftCellX := tableStartX
		isLeftSelected := a.selectedCell == leftCellNum
		
		// Draw cell background if selected (brighter and more visible)
		if isLeftSelected {
			// Draw filled rectangle for selection using multiple lines
			rectX := float64(leftCellX - 5)
			rectY := float64(rowY - cellHeight + 5)
			rectW := float64(cellWidth - 10)
			rectH := float64(cellHeight - 10)
			selectionColor := color.RGBA{150, 150, 255, 200}
			// Fill rectangle with horizontal lines
			for y := rectY; y < rectY+rectH; y++ {
				ebitenutil.DrawLine(screen, rectX, y, rectX+rectW, y, selectionColor)
			}
		}
		
		// Draw cell border (brighter if selected)
		var leftBorderColor color.Color = color.Gray{Y: 100}
		if isLeftSelected {
			leftBorderColor = color.RGBA{200, 200, 255, 255}
		}
		ebitenutil.DrawRect(screen, float64(leftCellX-5), float64(rowY-cellHeight+5), float64(cellWidth-10), float64(cellHeight-10), leftBorderColor)
		
		// Draw cell content: cell number, value, and next k value
		nextK := a.calculateNextKValue(leftValue)
		leftText := strconv.Itoa(leftCellNum) + ": " + strconv.Itoa(leftValue) + " (k=" + strconv.Itoa(nextK) + ")"
		text.Draw(screen, leftText, basicfont.Face7x13, leftCellX, rowY-10, color.White)
		
		// Draw right cell (positive) - displayed on right, but numbered first
		rightValue := a.getCellValue(rightCellNum)
		rightCellX := tableStartX + cellWidth
		isRightSelected := a.selectedCell == rightCellNum
		
		// Draw cell background if selected (brighter and more visible)
		if isRightSelected {
			// Draw filled rectangle for selection using multiple lines
			rectX := float64(rightCellX - 5)
			rectY := float64(rowY - cellHeight + 5)
			rectW := float64(cellWidth - 10)
			rectH := float64(cellHeight - 10)
			selectionColor := color.RGBA{150, 150, 255, 200}
			// Fill rectangle with horizontal lines
			for y := rectY; y < rectY+rectH; y++ {
				ebitenutil.DrawLine(screen, rectX, y, rectX+rectW, y, selectionColor)
			}
		}
		
		// Draw cell border (brighter if selected)
		var rightBorderColor color.Color = color.Gray{Y: 100}
		if isRightSelected {
			rightBorderColor = color.RGBA{200, 200, 255, 255}
		}
		ebitenutil.DrawRect(screen, float64(rightCellX-5), float64(rowY-cellHeight+5), float64(cellWidth-10), float64(cellHeight-10), rightBorderColor)
		
		// Draw cell content: cell number, value, and next k value
		nextK = a.calculateNextKValue(rightValue)
		rightText := strconv.Itoa(rightCellNum) + ": " + strconv.Itoa(rightValue) + " (k=" + strconv.Itoa(nextK) + ")"
		text.Draw(screen, rightText, basicfont.Face7x13, rightCellX, rowY-10, color.White)
	}
	
	// Draw history list
	if len(a.history) > 0 {
		historyY := screenHeight - 120
		historyText := "History: "
		for i, entry := range a.history {
			if i > 0 {
				historyText += ", "
			}
			historyText += "cell " + strconv.Itoa(entry.cellNum) + " (k=" + strconv.Itoa(entry.kValue) + ")"
		}
		// Truncate if too long
		if len(historyText) > 150 {
			historyText = historyText[:147] + "..."
		}
		text.Draw(screen, historyText, basicfont.Face7x13, 10, historyY, color.RGBA{200, 200, 255, 255})
	}
	
	// Draw generated sequence at the bottom if it exists (show 20 values starting from sequenceScrollOffset, using even numbers only)
	if a.usingGeneratedSequence {
		seqY := screenHeight - 90
		startX := 10
		spacing := 60 // Horizontal spacing between numbers
		numValues := 20 // Number of values to display
		
		// Draw sequence numbers on first line (using even numbers: -4, -2, 0, 2, 4, ...)
		for i := 0; i < numValues; i++ {
			// Calculate even index: sequenceScrollOffset + i*2
			// sequenceScrollOffset should be even, and we step by 2
			sequenceIndex := a.sequenceScrollOffset + i*2
			val := a.getSequenceValue(sequenceIndex)
			valText := strconv.Itoa(val)
			x := startX + i*spacing
			text.Draw(screen, valText, basicfont.Face7x13, x, seqY, color.RGBA{200, 200, 255, 255})
		}
		
		// Draw k values below each number
		for i := 0; i < numValues; i++ {
			sequenceIndex := a.sequenceScrollOffset + i*2
			val := a.getSequenceValue(sequenceIndex)
			kValue := a.calculateNextKValue(val)
			kText := "k=" + strconv.Itoa(kValue)
			x := startX + i*spacing
			text.Draw(screen, kText, basicfont.Face7x13, x, seqY+15, color.RGBA{150, 150, 200, 255})
		}
		
		// Draw offset and scroll info
		offsetText := "Offset: " + strconv.Itoa(a.sequenceOffset) + " | Scroll: " + strconv.Itoa(a.sequenceScrollOffset)
		text.Draw(screen, offsetText, basicfont.Face7x13, 10, seqY+30, color.RGBA{200, 200, 255, 255})
	}
	
	// Draw offset input dialogs if open
	if a.showOffsetDialog {
		a.drawOffsetInputDialog(screen)
	}
	if a.showScrollOffsetDialog {
		a.drawScrollOffsetInputDialog(screen)
	}
	
	// Draw help dialog if open
	if a.showHelp {
		a.drawHelpDialog(screen)
	}
	
	// Draw instructions (simplified - details in help dialog)
	instructions := "Arrow Keys: Navigate | Enter: Generate sequence | H: Help | ESC: Return"
	instBounds := text.BoundString(basicfont.Face7x13, instructions)
	instX := (screenWidth - instBounds.Dx()) / 2
	instY := screenHeight - 30
	text.Draw(screen, instructions, basicfont.Face7x13, instX, instY, color.Gray{Y: 150})
}

// drawOffsetInputDialog draws the offset input dialog
func (a *AlternatingJacobsthalChapter) drawOffsetInputDialog(screen *ebiten.Image) {
	// Draw semi-transparent overlay
	overlayColor := color.RGBA{0, 0, 0, 200}
	ebitenutil.DrawRect(screen, 0, 0, float64(screenWidth), float64(screenHeight), overlayColor)
	
	// Draw dialog box
	dialogWidth := 400.0
	dialogHeight := 120.0
	dialogX := (float64(screenWidth) - dialogWidth) / 2
	dialogY := (float64(screenHeight) - dialogHeight) / 2
	
	// Draw dialog background
	ebitenutil.DrawRect(screen, dialogX, dialogY, dialogWidth, dialogHeight, color.RGBA{40, 40, 50, 255})
	
	// Draw dialog border
	ebitenutil.DrawRect(screen, dialogX, dialogY, dialogWidth, 2, color.White)
	ebitenutil.DrawRect(screen, dialogX, dialogY, 2, dialogHeight, color.White)
	ebitenutil.DrawRect(screen, dialogX+dialogWidth-2, dialogY, 2, dialogHeight, color.White)
	ebitenutil.DrawRect(screen, dialogX, dialogY+dialogHeight-2, dialogWidth, 2, color.White)
	
	// Draw title
	titleText := "Set offset"
	titleBounds := text.BoundString(basicfont.Face7x13, titleText)
	titleX := int(dialogX + (dialogWidth-float64(titleBounds.Dx()))/2)
	titleY := int(dialogY + 20)
	text.Draw(screen, titleText, basicfont.Face7x13, titleX, titleY, color.White)
	
	// Draw input prompt
	promptText := "Offset: "
	promptBounds := text.BoundString(basicfont.Face7x13, promptText)
	promptX := int(dialogX + 20)
	promptY := int(dialogY + 50)
	text.Draw(screen, promptText, basicfont.Face7x13, promptX, promptY, color.White)
	
	// Draw input buffer
	inputX := promptX + promptBounds.Dx() + 5
	inputY := promptY
	inputText := a.offsetInputBuffer
	if inputText == "" {
		inputText = "_" // Cursor
	} else {
		inputText += "_" // Add cursor at end
	}
	text.Draw(screen, inputText, basicfont.Face7x13, inputX, inputY, color.RGBA{100, 255, 100, 255})
	
	// Draw instruction
	instText := "Press ENTER to confirm, ESC to cancel"
	instBounds := text.BoundString(basicfont.Face7x13, instText)
	instX := int(dialogX + (dialogWidth-float64(instBounds.Dx()))/2)
	instY := int(dialogY + 90)
	text.Draw(screen, instText, basicfont.Face7x13, instX, instY, color.RGBA{200, 200, 200, 255})
}

// drawScrollOffsetInputDialog draws the scroll offset input dialog
func (a *AlternatingJacobsthalChapter) drawScrollOffsetInputDialog(screen *ebiten.Image) {
	// Draw semi-transparent overlay
	overlayColor := color.RGBA{0, 0, 0, 200}
	ebitenutil.DrawRect(screen, 0, 0, float64(screenWidth), float64(screenHeight), overlayColor)
	
	// Draw dialog box
	dialogWidth := 400.0
	dialogHeight := 120.0
	dialogX := (float64(screenWidth) - dialogWidth) / 2
	dialogY := (float64(screenHeight) - dialogHeight) / 2
	
	// Draw dialog background
	ebitenutil.DrawRect(screen, dialogX, dialogY, dialogWidth, dialogHeight, color.RGBA{40, 40, 50, 255})
	
	// Draw dialog border
	ebitenutil.DrawRect(screen, dialogX, dialogY, dialogWidth, 2, color.White)
	ebitenutil.DrawRect(screen, dialogX, dialogY, 2, dialogHeight, color.White)
	ebitenutil.DrawRect(screen, dialogX+dialogWidth-2, dialogY, 2, dialogHeight, color.White)
	ebitenutil.DrawRect(screen, dialogX, dialogY+dialogHeight-2, dialogWidth, 2, color.White)
	
	// Draw title
	titleText := "Jump to sequence value"
	titleBounds := text.BoundString(basicfont.Face7x13, titleText)
	titleX := int(dialogX + (dialogWidth-float64(titleBounds.Dx()))/2)
	titleY := int(dialogY + 20)
	text.Draw(screen, titleText, basicfont.Face7x13, titleX, titleY, color.White)
	
	// Draw input prompt
	promptText := "Value: "
	promptBounds := text.BoundString(basicfont.Face7x13, promptText)
	promptX := int(dialogX + 20)
	promptY := int(dialogY + 50)
	text.Draw(screen, promptText, basicfont.Face7x13, promptX, promptY, color.White)
	
	// Draw input buffer
	inputX := promptX + promptBounds.Dx() + 5
	inputY := promptY
	inputText := a.scrollOffsetInputBuffer
	if inputText == "" {
		inputText = "_" // Cursor
	} else {
		inputText += "_" // Add cursor at end
	}
	text.Draw(screen, inputText, basicfont.Face7x13, inputX, inputY, color.RGBA{100, 255, 100, 255})
	
	// Draw instruction
	instText := "Press ENTER to confirm, ESC to cancel"
	instBounds := text.BoundString(basicfont.Face7x13, instText)
	instX := int(dialogX + (dialogWidth-float64(instBounds.Dx()))/2)
	instY := int(dialogY + 90)
	text.Draw(screen, instText, basicfont.Face7x13, instX, instY, color.RGBA{200, 200, 200, 255})
}

// drawHelpDialog draws the help dialog
func (a *AlternatingJacobsthalChapter) drawHelpDialog(screen *ebiten.Image) {
	// Draw semi-transparent overlay
	overlayColor := color.RGBA{0, 0, 0, 200}
	ebitenutil.DrawRect(screen, 0, 0, float64(screenWidth), float64(screenHeight), overlayColor)
	
	// Draw help dialog box
	dialogWidth := 600.0
	dialogHeight := 500.0
	dialogX := (float64(screenWidth) - dialogWidth) / 2
	dialogY := (float64(screenHeight) - dialogHeight) / 2
	
	// Draw dialog background
	ebitenutil.DrawRect(screen, dialogX, dialogY, dialogWidth, dialogHeight, color.RGBA{30, 30, 40, 255})
	
	// Draw dialog border
	ebitenutil.DrawRect(screen, dialogX, dialogY, dialogWidth, 2, color.White)
	ebitenutil.DrawRect(screen, dialogX, dialogY, 2, dialogHeight, color.White)
	ebitenutil.DrawRect(screen, dialogX+dialogWidth-2, dialogY, 2, dialogHeight, color.White)
	ebitenutil.DrawRect(screen, dialogX, dialogY+dialogHeight-2, dialogWidth, 2, color.White)
	
	// Draw title
	titleText := "HELP - KEYBOARD CONTROLS"
	titleBounds := text.BoundString(basicfont.Face7x13, titleText)
	titleX := int(dialogX + (dialogWidth-float64(titleBounds.Dx()))/2)
	titleY := int(dialogY + 20)
	text.Draw(screen, titleText, basicfont.Face7x13, titleX, titleY, color.White)
	
	// Help content lines
	helpLines := []string{
		"",
		"NAVIGATION:",
		"  Arrow Keys            - Navigate cells in the table",
		"",
		"SEQUENCE GENERATION:",
		"  Enter                 - Generate sequence from selected cell",
		"                       - Formula: jacobsthal_number + i * 2^cell_number",
		"",
		"OFFSET ADJUSTMENT:",
		"  Shift+Left/Right      - Adjust offset for sequence indexing",
		"  O                     - Open dialog to manually set offset",
		"  A                     - Auto-find offset to left (cells 1,2,3 match k values)",
		"  S                     - Auto-find offset to right (cells 1,2,3 match k values)",
		"",
		"SEQUENCE SCROLLING:",
		"  K/L                   - Scroll sequence left/right",
		"  P                     - Open dialog to jump to specific sequence value",
		"",
		"RESET:",
		"  R                     - Reset sequence and history",
		"",
		"DIALOGS:",
		"  O                     - Open offset input dialog",
		"  P                     - Open scroll position input dialog",
		"  Type number           - Enter value in dialog",
		"  Enter                 - Confirm dialog",
		"  ESC                   - Cancel dialog",
		"  Backspace             - Delete last character",
		"",
		"HELP:",
		"  H                     - Show/hide this help dialog",
		"  UP/DOWN or W/S        - Scroll help (when open)",
		"",
		"",
		"Press H or ESC to close",
	}
	
	// Draw scrollable content
	startY := int(dialogY) + 50 - a.helpScrollOffset
	lineHeight := 15
	
	for i, line := range helpLines {
		y := startY + i*lineHeight
		// Only draw visible lines
		if y >= int(dialogY)+40 && y <= int(dialogY)+int(dialogHeight)-30 {
			// Color code different sections
			var lineColor color.Color = color.Gray{Y: 200}
			if len(line) > 0 && line[0] != ' ' {
				// Section headers
				lineColor = color.White
			} else if len(line) > 2 && line[:2] == "  " {
				// Regular lines
				lineColor = color.Gray{Y: 180}
			}
			text.Draw(screen, line, basicfont.Face7x13, int(dialogX+20), y, lineColor)
		}
	}
	
	// Draw scroll indicator if content is scrollable
	totalHeight := len(helpLines) * lineHeight
	if totalHeight > int(dialogHeight-70) {
		// Show scroll position
		scrollText := fmt.Sprintf("Scroll: %d/%d", a.helpScrollOffset, totalHeight-int(dialogHeight-70))
		scrollBounds := text.BoundString(basicfont.Face7x13, scrollText)
		scrollX := int(dialogX + dialogWidth - float64(scrollBounds.Dx()) - 10)
		scrollY := int(dialogY + dialogHeight - 20)
		text.Draw(screen, scrollText, basicfont.Face7x13, scrollX, scrollY, color.Gray{Y: 120})
	}
}
