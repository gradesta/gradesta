package main

import (
	"image/color"
	"math"
	"math/big"
	"strconv"

	"github.com/hajimehoshi/ebiten/v2"
	"github.com/hajimehoshi/ebiten/v2/ebitenutil"
	"github.com/hajimehoshi/ebiten/v2/inpututil"
	"github.com/hajimehoshi/ebiten/v2/text"
	"golang.org/x/image/font/basicfont"
)

// ThreeRowBootlaceChapter implements the Three Row Bootlace chapter
type ThreeRowBootlaceChapter struct {
	selectedColumn int // 0 = 1 MOD 3, 1 = 0 MOD 3, 2 = 2 MOD 3
	selectedRow    int // Currently selected row index
	scrollOffset   int // Vertical scroll offset (row index, center of visible area)
	escConsumed    bool
	keyRepeatFrame int // Frame counter for key repeat
}

// NewThreeRowBootlaceChapter creates a new Three Row Bootlace chapter
func NewThreeRowBootlaceChapter() *ThreeRowBootlaceChapter {
	return &ThreeRowBootlaceChapter{
		selectedColumn: 1, // Start in middle column
		selectedRow:    0, // Start at row 0
		scrollOffset:   0, // Start scroll at row 0
	}
}

// getOddNumberForColumn returns the odd number at the given row index in the given column
// Row 0 is the center, positive rows go down, negative rows go up
// Column 0 = 1 MOD 3, Column 1 = 0 MOD 3, Column 2 = 2 MOD 3
func (t *ThreeRowBootlaceChapter) getOddNumberForColumn(column int, row int) int {
	// For each column, we need to find the pattern of odd numbers
	// Column 0 (1 MOD 3): 1, 7, 13, 19, 25, ... (positive) and -5, -11, -17, -23, ... (negative)
	// Column 1 (0 MOD 3): 3, 9, 15, 21, 27, ... (positive) and -3, -9, -15, -21, ... (negative)
	// Column 2 (2 MOD 3): 5, 11, 17, 23, 29, ... (positive) and -1, -7, -13, -19, ... (negative)
	
	// The pattern: for column c, positive numbers start at (2*c + 1) and increment by 6
	// For negative numbers, we need to find the pattern
	
	var base int
	var increment int = 6
	
	switch column {
	case 0: // 1 MOD 3
		base = 1
	case 1: // 0 MOD 3
		base = 3
	case 2: // 2 MOD 3
		base = 5
	}
	
	if row >= 0 {
		// Positive direction: base + row * 6
		return base + row*increment
	} else {
		// Negative direction: base - 6 * |row|
		// For column 0: row -1 = 1 - 6 = -5, row -2 = 1 - 12 = -11
		// For column 1: row -1 = 3 - 6 = -3, row -2 = 3 - 12 = -9
		// For column 2: row -1 = 5 - 6 = -1, row -2 = 5 - 12 = -7
		return base - 6*(-row)
	}
}

// getCollatzNext calculates the next number in the Collatz sequence for an odd number n
// Returns (nextNumber, k) where k is the number of times divided by 2
func (t *ThreeRowBootlaceChapter) getCollatzNext(n int) (int, int) {
	// Collatz: (3n + 1) / 2^k, where k is the number of times the result is divisible by 2
	// Use big.Int for large numbers
	nBig := big.NewInt(int64(n))
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
	
	nextNum := int(result.Int64())
	return nextNum, k
}

// findPredecessors finds all odd numbers that lead to the given number via Collatz
// Returns a slice of (column, row, number) tuples
func (t *ThreeRowBootlaceChapter) findPredecessors(targetNum int) []struct {
	col  int
	row  int
	num  int
} {
	var predecessors []struct {
		col int
		row int
		num int
	}
	
	// For a number n, predecessors are numbers m where (3m + 1) / 2^k = n
	// This means: 3m + 1 = n * 2^k, so m = (n * 2^k - 1) / 3
	// We need m to be odd and an integer
	
	// Try different k values (powers of 2)
	for k := 1; k <= 20; k++ {
		// Calculate potential predecessor: m = (n * 2^k - 1) / 3
		powerOf2 := 1 << uint(k) // 2^k
		numerator := targetNum*powerOf2 - 1
		
		// Check if divisible by 3
		if numerator%3 == 0 {
			m := numerator / 3
			
			// Check if m is odd
			if m%2 != 0 {
				// Find which column and row this number is in
				for col := 0; col < 3; col++ {
					row, found := t.findRowForNumber(col, m)
					if found {
						predecessors = append(predecessors, struct {
							col int
							row int
							num int
						}{col: col, row: row, num: m})
						break // Found in this column, no need to check others
					}
				}
			}
		}
	}
	
	return predecessors
}

// findRowForNumber finds the row index in the given column for a specific odd number
// Returns (row, found) where found is true if the number exists in that column
func (t *ThreeRowBootlaceChapter) findRowForNumber(column int, targetNum int) (int, bool) {
	// Check if the number has the correct mod 3 value
	var expectedMod int
	switch column {
	case 0:
		expectedMod = 1
	case 1:
		expectedMod = 0
	case 2:
		expectedMod = 2
	}
	
	// Handle negative numbers correctly
	mod := targetNum % 3
	if mod < 0 {
		mod += 3
	}
	if mod != expectedMod {
		return 0, false
	}
	
	// Find the row
	var base int
	switch column {
	case 0:
		base = 1
	case 1:
		base = 3
	case 2:
		base = 5
	}
	
	// Calculate row: (targetNum - base) / 6
	diff := targetNum - base
	if diff%6 != 0 {
		return 0, false // Not in the sequence
	}
	
	row := diff / 6
	return row, true
}

// Update updates the Three Row Bootlace chapter
func (t *ThreeRowBootlaceChapter) Update() error {
	t.escConsumed = false
	
	// Increment key repeat frame counter
	t.keyRepeatFrame++
	
	// Calculate how many rows are visible based on screen height
	// Dots start at Y=80, and we need space at the bottom for instructions (30 pixels)
	// Each row is 50 pixels apart
	availableHeight := screenHeight - 80 - 30
	visibleRows := availableHeight / 50
	if visibleRows < 1 {
		visibleRows = 1
	}
	
	// Helper function to ensure a row is visible (only scrolls if out of view)
	ensureRowVisible := func(row int) {
		// Recalculate visible range based on current scroll offset
		// The visible range is from startRow (inclusive) to endRow (inclusive)
		startRow := t.scrollOffset - visibleRows/2
		endRow := t.scrollOffset + visibleRows/2
		
		// Check if row is outside the visible range
		// Use strict comparisons: if row is before startRow or after endRow, it's out of view
		if row < startRow {
			// Row is above the top of visible area, scroll up to center it
			t.scrollOffset = row + visibleRows/2
		} else if row > endRow {
			// Row is below the bottom of visible area, scroll down to center it
			t.scrollOffset = row - visibleRows/2
		}
		// If row is within the visible range (startRow <= row <= endRow), don't change scrollOffset
	}
	
	// Handle arrow keys for navigation (support holding down keys)
	// Use a combination: IsKeyJustPressed for immediate response, IsKeyPressed with frame counter for repeat
	keyRepeatDelay := 10 // Frames to wait before repeating (adjust for speed)
	shouldMove := false
	
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowLeft) {
		t.selectedColumn--
		if t.selectedColumn < 0 {
			t.selectedColumn = 2
		}
		shouldMove = true
		t.keyRepeatFrame = 0 // Reset counter on new key press
	} else if ebiten.IsKeyPressed(ebiten.KeyArrowLeft) && t.keyRepeatFrame >= keyRepeatDelay {
		t.selectedColumn--
		if t.selectedColumn < 0 {
			t.selectedColumn = 2
		}
		shouldMove = true
		t.keyRepeatFrame = 0 // Reset counter after movement
	}
	
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowRight) {
		t.selectedColumn++
		if t.selectedColumn > 2 {
			t.selectedColumn = 0
		}
		shouldMove = true
		t.keyRepeatFrame = 0
	} else if ebiten.IsKeyPressed(ebiten.KeyArrowRight) && t.keyRepeatFrame >= keyRepeatDelay {
		t.selectedColumn++
		if t.selectedColumn > 2 {
			t.selectedColumn = 0
		}
		shouldMove = true
		t.keyRepeatFrame = 0
	}
	
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowUp) {
		t.selectedRow--
		shouldMove = true
		t.keyRepeatFrame = 0
	} else if ebiten.IsKeyPressed(ebiten.KeyArrowUp) && t.keyRepeatFrame >= keyRepeatDelay {
		t.selectedRow--
		shouldMove = true
		t.keyRepeatFrame = 0
	}
	
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowDown) {
		t.selectedRow++
		shouldMove = true
		t.keyRepeatFrame = 0
	} else if ebiten.IsKeyPressed(ebiten.KeyArrowDown) && t.keyRepeatFrame >= keyRepeatDelay {
		t.selectedRow++
		shouldMove = true
		t.keyRepeatFrame = 0
	}
	
	// Check if selected row is visible and scroll if needed (only after movement)
	if shouldMove {
		ensureRowVisible(t.selectedRow)
	}
	
	// Check if shift is pressed
	shiftPressed := ebiten.IsKeyPressed(ebiten.KeyShiftLeft) || ebiten.IsKeyPressed(ebiten.KeyShiftRight)
	
	// Handle space bar to jump to next Collatz number (forward)
	if inpututil.IsKeyJustPressed(ebiten.KeySpace) && !shiftPressed {
		currentNum := t.getOddNumberForColumn(t.selectedColumn, t.selectedRow)
		nextNum, k := t.getCollatzNext(currentNum)
		
		if k > 0 {
			// Find which column and row the next number is in
			for nextCol := 0; nextCol < 3; nextCol++ {
				nextRow, found := t.findRowForNumber(nextCol, nextNum)
				if found {
					t.selectedColumn = nextCol
					t.selectedRow = nextRow
					// Ensure the destination is visible
					ensureRowVisible(t.selectedRow)
					break
				}
			}
		}
	}
	
	// Handle Shift+Space to jump to previous Collatz number (backwards)
	if inpututil.IsKeyJustPressed(ebiten.KeySpace) && shiftPressed {
		currentNum := t.getOddNumberForColumn(t.selectedColumn, t.selectedRow)
		predecessors := t.findPredecessors(currentNum)
		
		if len(predecessors) > 0 {
			// Jump to the first predecessor found
			// (In the future, could allow cycling through multiple predecessors)
			pred := predecessors[0]
			t.selectedColumn = pred.col
			t.selectedRow = pred.row
			// Ensure the destination is visible
			ensureRowVisible(t.selectedRow)
		}
	}
	
	return nil
}

// Draw draws the Three Row Bootlace chapter
func (t *ThreeRowBootlaceChapter) Draw(screen *ebiten.Image) {
	// Draw title
	titleText := "Three Row Bootlace"
	titleBounds := text.BoundString(basicfont.Face7x13, titleText)
	titleX := (screenWidth - titleBounds.Dx()) / 2
	titleY := 20
	text.Draw(screen, titleText, basicfont.Face7x13, titleX, titleY, color.White)
	
	// Column positions
	colWidth := screenWidth / 3
	col0X := colWidth / 2      // 1 MOD 3
	col1X := colWidth + colWidth/2 // 0 MOD 3
	col2X := 2*colWidth + colWidth/2 // 2 MOD 3
	
	// Column labels
	labelY := 50
	text.Draw(screen, "1 MOD 3", basicfont.Face7x13, col0X-30, labelY, color.White)
	text.Draw(screen, "0 MOD 3", basicfont.Face7x13, col1X-30, labelY, color.White)
	text.Draw(screen, "2 MOD 3", basicfont.Face7x13, col2X-30, labelY, color.White)
	
	// Calculate visible range based on screen height
	// Dots start at Y=80, and we need space at the bottom for instructions (30 pixels)
	// Each row is 50 pixels apart
	availableHeight := screenHeight - 80 - 30
	visibleRows := availableHeight / 50
	if visibleRows < 1 {
		visibleRows = 1
	}
	startRow := t.scrollOffset - visibleRows/2
	endRow := t.scrollOffset + visibleRows/2
	
	// First pass: draw all lines (so they appear behind dots)
	// Store selected dot arrow info to draw on top later
	var selectedArrowInfo *struct {
		fromX, fromY, toX, toY float64
		dirX, dirY            float64
	}
	
	for col := 0; col < 3; col++ {
		for row := startRow; row <= endRow; row++ {
			num := t.getOddNumberForColumn(col, row)
			nextNum, k := t.getCollatzNext(num)
			
			if k > 0 {
				// Find which column and row the next number is in
				for nextCol := 0; nextCol < 3; nextCol++ {
					nextRow, found := t.findRowForNumber(nextCol, nextNum)
					if found {
						// Calculate positions
						dotY := 80 + (row-startRow)*50
						
						var fromX, toX int
						switch col {
						case 0:
							fromX = col0X
						case 1:
							fromX = col1X
						case 2:
							fromX = col2X
						}
						switch nextCol {
						case 0:
							toX = col0X
						case 1:
							toX = col1X
						case 2:
							toX = col2X
						}
						
						// Calculate destination Y (even if not visible, we'll draw in the general direction)
						var nextDotY float64
						if nextRow >= startRow && nextRow <= endRow {
							// Destination is visible, use actual position
							nextDotY = 80 + float64((nextRow-startRow)*50)
						} else {
							// Destination is far away, calculate virtual dot position
							// Use virtual dots that are farther above/below the screen
							if nextRow < startRow {
								// Destination is above visible area
								// Place virtual dot above the top of the screen
								// Calculate how far above: use a fixed offset above the top edge
								virtualOffset := -100.0 // 100 pixels above the top edge
								nextDotY = 80 + virtualOffset
							} else {
								// Destination is below visible area
								// Place virtual dot below the bottom of the screen
								// Calculate how far below: use a fixed offset below the bottom edge
								virtualOffset := float64(screenHeight - 30 - 80) + 100.0 // 100 pixels below the bottom edge
								nextDotY = 80 + virtualOffset
							}
						}
						
						// Check if this is the selected dot's arrow
						isSelected := (col == t.selectedColumn && row == t.selectedRow)
						
						// Skip drawing selected arrow now, will draw on top later
						if isSelected {
							// Calculate direction
							dx := float64(toX - fromX)
							dy := nextDotY - float64(dotY)
							length := math.Sqrt(dx*dx + dy*dy)
							if length > 0 {
								dirX := dx / length
								dirY := dy / length
								selectedArrowInfo = &struct {
									fromX, fromY, toX, toY float64
									dirX, dirY            float64
								}{
									fromX: float64(fromX),
									fromY: float64(dotY),
									toX:   float64(toX),
									toY:   nextDotY,
									dirX:  dirX,
									dirY:  dirY,
								}
							}
							continue
						}
						
						// Draw line with arrow (for non-selected dots)
						// Stop the line short of the destination (8 pixels before the dot)
						dx := float64(toX - fromX)
						dy := nextDotY - float64(dotY)
						length := math.Sqrt(dx*dx + dy*dy)
						
						if length > 0 {
							// Normalize direction vector
							dirX := dx / length
							dirY := dy / length
							
							// Calculate end point (8 pixels before destination)
							arrowLength := 8.0
							endX := float64(toX) - dirX*arrowLength
							endY := float64(nextDotY) - dirY*arrowLength
							
							// Draw the line
							ebitenutil.DrawLine(screen, float64(fromX), float64(dotY), endX, endY, color.RGBA{100, 100, 100, 255})
							
							// Draw arrowhead (triangle pointing in direction of travel)
							arrowSize := 6.0
							
							// Calculate arrowhead points (triangle)
							// Tip of arrow
							tipX := endX
							tipY := endY
							
							// Base of arrow (perpendicular to direction)
							perpX := -dirY
							perpY := dirX
							
							// Left point of arrow base
							leftX := endX - dirX*arrowSize + perpX*arrowSize*0.5
							leftY := endY - dirY*arrowSize + perpY*arrowSize*0.5
							
							// Right point of arrow base
							rightX := endX - dirX*arrowSize - perpX*arrowSize*0.5
							rightY := endY - dirY*arrowSize - perpY*arrowSize*0.5
							
							// Draw arrowhead as filled triangle using small rectangles
							arrowColor := color.RGBA{100, 100, 100, 255}
							
							// Draw the three edges of the triangle
							ebitenutil.DrawLine(screen, tipX, tipY, leftX, leftY, arrowColor)
							ebitenutil.DrawLine(screen, tipX, tipY, rightX, rightY, arrowColor)
							ebitenutil.DrawLine(screen, leftX, leftY, rightX, rightY, arrowColor)
							
							// Fill the triangle by drawing small rectangles
							// Find bounding box
							minX := math.Min(tipX, math.Min(leftX, rightX))
							maxX := math.Max(tipX, math.Max(leftX, rightX))
							minY := math.Min(tipY, math.Min(leftY, rightY))
							maxY := math.Max(tipY, math.Max(leftY, rightY))
							
							// Simple point-in-triangle check
							// For each pixel in bounding box, check if it's inside triangle
							for y := minY; y <= maxY; y += 1.0 {
								for x := minX; x <= maxX; x += 1.0 {
									// Check if point (x, y) is inside triangle using barycentric coordinates
									v0x := rightX - tipX
									v0y := rightY - tipY
									v1x := leftX - tipX
									v1y := leftY - tipY
									v2x := x - tipX
									v2y := y - tipY
									
									dot00 := v0x*v0x + v0y*v0y
									dot01 := v0x*v1x + v0y*v1y
									dot02 := v0x*v2x + v0y*v2y
									dot11 := v1x*v1x + v1y*v1y
									dot12 := v1x*v2x + v1y*v2y
									
									invDenom := 1 / (dot00*dot11 - dot01*dot01)
									u := (dot11*dot02 - dot01*dot12) * invDenom
									v := (dot00*dot12 - dot01*dot02) * invDenom
									
									if u >= 0 && v >= 0 && u+v <= 1 {
										ebitenutil.DrawRect(screen, x-0.5, y-0.5, 1, 1, arrowColor)
									}
								}
							}
						}
						break
					}
				}
			}
		}
	}
	
	// Second pass: draw dots and labels
	for col := 0; col < 3; col++ {
		for row := startRow; row <= endRow; row++ {
			num := t.getOddNumberForColumn(col, row)
			_, k := t.getCollatzNext(num)
			
			dotY := 80 + (row-startRow)*50
			
			var dotX int
			switch col {
			case 0:
				dotX = col0X
			case 1:
				dotX = col1X
			case 2:
				dotX = col2X
			}
			
			// Draw selection highlight
			if col == t.selectedColumn && row == t.selectedRow {
				// Draw filled circle for selected dot
				ebitenutil.DrawCircle(screen, float64(dotX), float64(dotY), 8, color.RGBA{100, 150, 255, 255})
			}
			
			// Draw dot
			var dotColor color.Color = color.White
			if col == t.selectedColumn && row == t.selectedRow {
				dotColor = color.RGBA{200, 220, 255, 255}
			}
			ebitenutil.DrawCircle(screen, float64(dotX), float64(dotY), 5, dotColor)
			
			// Draw k value label (to the left of the dot)
			kText := strconv.Itoa(k)
			kBounds := text.BoundString(basicfont.Face7x13, kText)
			kX := dotX - kBounds.Dx() - 10 // 10 pixels to the left
			kY := dotY
			text.Draw(screen, kText, basicfont.Face7x13, kX, kY, color.RGBA{200, 200, 200, 255})
			
			// Draw number label (to the right of the dot)
			numText := strconv.Itoa(num)
			numX := dotX + 10 // 10 pixels to the right
			numY := dotY
			text.Draw(screen, numText, basicfont.Face7x13, numX, numY, color.White)
		}
	}
	
	// Draw selected dot's arrow on top (after all other arrows and dots)
	if selectedArrowInfo != nil {
		// Draw the selected arrow in a bolder, different color
		selectedArrowColor := color.RGBA{100, 255, 255, 255} // Bright cyan - much more visible
		
		// Calculate end point (8 pixels before destination)
		arrowLength := 8.0
		endX := selectedArrowInfo.toX - selectedArrowInfo.dirX*arrowLength
		endY := selectedArrowInfo.toY - selectedArrowInfo.dirY*arrowLength
		
		// Draw the line (thicker for selected)
		ebitenutil.DrawLine(screen, selectedArrowInfo.fromX, selectedArrowInfo.fromY, endX, endY, selectedArrowColor)
		
		// Draw arrowhead (triangle pointing in direction of travel)
		arrowSize := 8.0 // Slightly larger for selected
		
		// Calculate arrowhead points (triangle)
		tipX := endX
		tipY := endY
		
		// Base of arrow (perpendicular to direction)
		perpX := -selectedArrowInfo.dirY
		perpY := selectedArrowInfo.dirX
		
		// Left point of arrow base
		leftX := endX - selectedArrowInfo.dirX*arrowSize + perpX*arrowSize*0.5
		leftY := endY - selectedArrowInfo.dirY*arrowSize + perpY*arrowSize*0.5
		
		// Right point of arrow base
		rightX := endX - selectedArrowInfo.dirX*arrowSize - perpX*arrowSize*0.5
		rightY := endY - selectedArrowInfo.dirY*arrowSize - perpY*arrowSize*0.5
		
		// Draw the three edges of the triangle
		ebitenutil.DrawLine(screen, tipX, tipY, leftX, leftY, selectedArrowColor)
		ebitenutil.DrawLine(screen, tipX, tipY, rightX, rightY, selectedArrowColor)
		ebitenutil.DrawLine(screen, leftX, leftY, rightX, rightY, selectedArrowColor)
		
		// Fill the triangle by drawing small rectangles
		minX := math.Min(tipX, math.Min(leftX, rightX))
		maxX := math.Max(tipX, math.Max(leftX, rightX))
		minY := math.Min(tipY, math.Min(leftY, rightY))
		maxY := math.Max(tipY, math.Max(leftY, rightY))
		
		// Simple point-in-triangle check
		for y := minY; y <= maxY; y += 1.0 {
			for x := minX; x <= maxX; x += 1.0 {
				// Check if point (x, y) is inside triangle using barycentric coordinates
				v0x := rightX - tipX
				v0y := rightY - tipY
				v1x := leftX - tipX
				v1y := leftY - tipY
				v2x := x - tipX
				v2y := y - tipY
				
				dot00 := v0x*v0x + v0y*v0y
				dot01 := v0x*v1x + v0y*v1y
				dot02 := v0x*v2x + v0y*v2y
				dot11 := v1x*v1x + v1y*v1y
				dot12 := v1x*v2x + v1y*v2y
				
				invDenom := 1 / (dot00*dot11 - dot01*dot01)
				u := (dot11*dot02 - dot01*dot12) * invDenom
				v := (dot00*dot12 - dot01*dot02) * invDenom
				
				if u >= 0 && v >= 0 && u+v <= 1 {
					ebitenutil.DrawRect(screen, x-0.5, y-0.5, 1, 1, selectedArrowColor)
				}
			}
		}
	}
	
	// Draw instructions
	instructions := "Arrow Keys: Navigate | Space: Jump forward | Shift+Space: Jump backward | ESC: Return"
	instBounds := text.BoundString(basicfont.Face7x13, instructions)
	instX := (screenWidth - instBounds.Dx()) / 2
	instY := screenHeight - 30
	text.Draw(screen, instructions, basicfont.Face7x13, instX, instY, color.Gray{Y: 150})
}

// WasEscConsumed returns whether ESC was consumed by a dialog this frame
func (t *ThreeRowBootlaceChapter) WasEscConsumed() bool {
	return t.escConsumed
}
