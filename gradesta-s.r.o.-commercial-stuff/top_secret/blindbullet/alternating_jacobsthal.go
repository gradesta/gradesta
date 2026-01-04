package main

import (
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
	
	// Check if shift is pressed for offset adjustment
	shiftPressed := ebiten.IsKeyPressed(ebiten.KeyShiftLeft) || ebiten.IsKeyPressed(ebiten.KeyShiftRight)
	
	// Handle Shift+Left/Right to adjust sequence offset (check this first)
	if shiftPressed {
		if inpututil.IsKeyJustPressed(ebiten.KeyArrowLeft) || inpututil.IsKeyJustPressed(ebiten.KeyA) {
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
		if inpututil.IsKeyJustPressed(ebiten.KeyArrowLeft) || inpututil.IsKeyJustPressed(ebiten.KeyA) {
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
	
	// Draw instructions
	instructions := "Arrow Keys: Navigate | Enter: Generate sequence | Shift+Left/Right: Adjust offset | K/L: Scroll sequence | R: Reset | ESC: Return"
	instBounds := text.BoundString(basicfont.Face7x13, instructions)
	instX := (screenWidth - instBounds.Dx()) / 2
	instY := screenHeight - 30
	text.Draw(screen, instructions, basicfont.Face7x13, instX, instY, color.Gray{Y: 150})
}

