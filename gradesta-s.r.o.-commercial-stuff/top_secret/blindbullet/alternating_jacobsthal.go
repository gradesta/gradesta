package main

import (
	"image/color"
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
	
	escConsumed bool // Whether ESC was consumed by a dialog this frame
}

// NewAlternatingJacobsthalChapter creates a new Alternating Jacobsthal chapter
func NewAlternatingJacobsthalChapter() *AlternatingJacobsthalChapter {
	return &AlternatingJacobsthalChapter{
		selectedCell:  0,
		scrollOffset:  0,
		escConsumed:   false,
	}
}

// jacobsthal calculates the n-th Jacobsthal number
// J(0) = 0, J(1) = 1, J(n) = J(n-1) + 2*J(n-2)
func jacobsthal(n int) int {
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

// getCellValue returns the value for a given cell number
// Cell numbering: right to left, top to bottom
// Row 0: right cell = 0, left cell = 1
// Row 1: right cell = 2, left cell = 3
// Row 2: right cell = 4, left cell = 5
// But displayed as: left, right (so we see "1, 0 / 3, 2 / 5, 4")
func (a *AlternatingJacobsthalChapter) getCellValue(cellNum int) int {
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
	
	// Handle arrow key navigation
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
	
	// Draw column headers
	leftHeaderX := tableStartX
	rightHeaderX := tableStartX + cellWidth
	headerY := tableStartY - 20
	text.Draw(screen, "Negative", basicfont.Face7x13, leftHeaderX, headerY, color.Gray{Y: 200})
	text.Draw(screen, "Positive", basicfont.Face7x13, rightHeaderX, headerY, color.Gray{Y: 200})
	
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
		
		// Draw cell content: cell number and value
		leftText := strconv.Itoa(leftCellNum) + ": " + strconv.Itoa(leftValue)
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
		
		// Draw cell content: cell number and value
		rightText := strconv.Itoa(rightCellNum) + ": " + strconv.Itoa(rightValue)
		text.Draw(screen, rightText, basicfont.Face7x13, rightCellX, rowY-10, color.White)
	}
	
	// Draw instructions
	instructions := "Arrow Keys: Navigate | ESC: Return to menu"
	instBounds := text.BoundString(basicfont.Face7x13, instructions)
	instX := (screenWidth - instBounds.Dx()) / 2
	instY := screenHeight - 30
	text.Draw(screen, instructions, basicfont.Face7x13, instX, instY, color.Gray{Y: 150})
}

