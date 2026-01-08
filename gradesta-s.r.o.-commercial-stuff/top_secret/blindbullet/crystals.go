package main

import (
	"image/color"
	"math"
	"sort"
	"strconv"

	"github.com/hajimehoshi/ebiten/v2"
	"github.com/hajimehoshi/ebiten/v2/inpututil"
	"github.com/hajimehoshi/ebiten/v2/text"
	"golang.org/x/image/font/basicfont"
)

// SparseRow represents a binary number as sparse positions where 1s are located
// Positions are sorted in descending order (most significant first)
type SparseRow struct {
	positions []int
}

// NewSparseRow creates a new SparseRow with the given positions
func NewSparseRow(positions []int) *SparseRow {
	pos := make([]int, len(positions))
	copy(pos, positions)
	sort.Sort(sort.Reverse(sort.IntSlice(pos))) // Sort descending
	return &SparseRow{positions: pos}
}

// HasBitAt checks if there's a 1 at the given position
func (sr *SparseRow) HasBitAt(position int) bool {
	for _, p := range sr.positions {
		if p == position {
			return true
		}
	}
	return false
}

// FlipBit flips the bit at the given position
func (sr *SparseRow) FlipBit(position int) {
	idx := -1
	for i, p := range sr.positions {
		if p == position {
			idx = i
			break
		}
	}
	if idx == -1 {
		// Add a 1 at this position
		sr.positions = append(sr.positions, position)
		sort.Sort(sort.Reverse(sort.IntSlice(sr.positions))) // Keep sorted
	} else {
		// Remove the 1 at this position
		sr.positions = append(sr.positions[:idx], sr.positions[idx+1:]...)
	}
}

// ShiftLeft shifts left by n positions (multiply by 2^n)
func (sr *SparseRow) ShiftLeft(n int) *SparseRow {
	newPositions := make([]int, len(sr.positions))
	for i, p := range sr.positions {
		newPositions[i] = p - n
	}
	return NewSparseRow(newPositions)
}

// ShiftRight shifts right by n positions (divide by 2^n)
func (sr *SparseRow) ShiftRight(n int) *SparseRow {
	newPositions := make([]int, len(sr.positions))
	for i, p := range sr.positions {
		newPositions[i] = p + n
	}
	return NewSparseRow(newPositions)
}

// IsZero checks if the row is zero (no 1s)
func (sr *SparseRow) IsZero() bool {
	return len(sr.positions) == 0
}

// GetRightmostPosition returns the rightmost (least significant) position
func (sr *SparseRow) GetRightmostPosition() int {
	if len(sr.positions) == 0 {
		return 0
	}
	max := sr.positions[0]
	for _, p := range sr.positions {
		if p > max {
			max = p
		}
	}
	return max
}

// GetLeftmostPosition returns the leftmost (most significant) position
func (sr *SparseRow) GetLeftmostPosition() int {
	if len(sr.positions) == 0 {
		return 0
	}
	min := sr.positions[0]
	for _, p := range sr.positions {
		if p < min {
			min = p
		}
	}
	return min
}

// Clone creates a copy of this row
func (sr *SparseRow) Clone() *SparseRow {
	return NewSparseRow(sr.positions)
}

// Equals checks if this row equals another row
func (sr *SparseRow) Equals(other *SparseRow) bool {
	if len(sr.positions) != len(other.positions) {
		return false
	}
	for i, p := range sr.positions {
		if p != other.positions[i] {
			return false
		}
	}
	return true
}

// ToDecimal converts to decimal value
func (sr *SparseRow) ToDecimal() int {
	result := 0
	for _, pos := range sr.positions {
		result += int(math.Pow(2, float64(pos)))
	}
	return result
}

// Add adds two sparse rows together with carry propagation
func AddSparseRows(row1, row2 *SparseRow) *SparseRow {
	if row1.IsZero() {
		return row2.Clone()
	}
	if row2.IsZero() {
		return row1.Clone()
	}

	allPositions := append(row1.positions, row2.positions...)
	minPos := allPositions[0]
	maxPos := allPositions[0]
	for _, p := range allPositions {
		if p < minPos {
			minPos = p
		}
		if p > maxPos {
			maxPos = p
		}
	}

	length := maxPos - minPos + 3 // +3 for extra carry space
	dense1 := make([]int, length)
	dense2 := make([]int, length)

	for _, pos := range row1.positions {
		idx := pos - minPos + 1
		if idx >= 0 && idx < length {
			dense1[idx] = 1
		}
	}

	for _, pos := range row2.positions {
		idx := pos - minPos + 1
		if idx >= 0 && idx < length {
			dense2[idx] = 1
		}
	}

	result := make([]int, length)
	carry := 0

	for i := length - 1; i >= 0; i-- {
		sum := dense1[i] + dense2[i] + carry
		result[i] = sum % 2
		carry = sum / 2
	}

	resultPositions := []int{}
	for i := 0; i < length; i++ {
		if result[i] == 1 {
			pos := minPos - 1 + i
			resultPositions = append(resultPositions, pos)
		}
	}

	return NewSparseRow(resultPositions)
}

// AddNoCarry adds two sparse rows without carry (XOR-like behavior)
func AddSparseRowsNoCarry(row1, row2 *SparseRow) *SparseRow {
	combinedPositions := append(row1.positions, row2.positions...)
	positionCounts := make(map[int]int)

	for _, pos := range combinedPositions {
		positionCounts[pos]++
	}

	resultPositions := []int{}
	for pos, count := range positionCounts {
		if count%2 == 1 {
			resultPositions = append(resultPositions, pos)
		}
	}

	return NewSparseRow(resultPositions)
}

// AddWithCarries adds sparse rows and returns carry information
type CarryInfo struct {
	Result  *SparseRow
	Carries map[int]string // position -> "plus-one" or "later"
}

func AddWithCarries(row1, row2, plusOneRow *SparseRow) CarryInfo {
	allPositions := append(row1.positions, row2.positions...)
	if plusOneRow != nil {
		allPositions = append(allPositions, plusOneRow.positions...)
	}

	if len(allPositions) == 0 {
		return CarryInfo{
			Result:  NewSparseRow([]int{}),
			Carries: make(map[int]string),
		}
	}

	minPos := allPositions[0]
	maxPos := allPositions[0]
	for _, p := range allPositions {
		if p < minPos {
			minPos = p
		}
		if p > maxPos {
			maxPos = p
		}
	}

	length := maxPos - minPos + 3
	dense1 := make([]int, length)
	dense2 := make([]int, length)
	densePlusOne := make([]int, length)

	for _, pos := range row1.positions {
		idx := pos - minPos + 1
		if idx >= 0 && idx < length {
			dense1[idx] = 1
		}
	}

	for _, pos := range row2.positions {
		idx := pos - minPos + 1
		if idx >= 0 && idx < length {
			dense2[idx] = 1
		}
	}

	if plusOneRow != nil {
		for _, pos := range plusOneRow.positions {
			idx := pos - minPos + 1
			if idx >= 0 && idx < length {
				densePlusOne[idx] = 1
			}
		}
	}

	result := make([]int, length)
	carries := make(map[int]string)
	carry := 0
	plusOneCarryChain := false

	for i := length - 1; i >= 0; i-- {
		sum := dense1[i] + dense2[i] + densePlusOne[i] + carry
		result[i] = sum % 2
		newCarry := sum / 2

		if newCarry > 0 {
			pos := minPos - 1 + i
			if densePlusOne[i] == 1 && !plusOneCarryChain {
				plusOneCarryChain = true
				carries[pos] = "plus-one"
			} else if plusOneCarryChain {
				carries[pos] = "plus-one"
			} else {
				carries[pos] = "later"
			}
		}

		carry = newCarry
	}

	resultPositions := []int{}
	for i := 0; i < length; i++ {
		if result[i] == 1 {
			pos := minPos - 1 + i
			resultPositions = append(resultPositions, pos)
		}
	}

	return CarryInfo{
		Result:  NewSparseRow(resultPositions),
		Carries: carries,
	}
}

// CreatePlusOne creates a row with a single 1 at the rightmost position
func CreatePlusOne(rightmostPosition int) *SparseRow {
	return NewSparseRow([]int{rightmostPosition})
}

// CrystalsChapter implements the Crystals chapter
type CrystalsChapter struct {
	escConsumed bool

	// Grid state
	rows              []*SparseRow
	carries           []map[int]string // Carry information for each row
	currentRow        int              // Current row index (for cursor)
	currentCol        int              // Current column position
	scrollOffset      int              // Vertical scroll offset
	horizontalOffset  int              // Horizontal scroll offset
	visibleRows       int              // Number of rows visible
	visibleCols       int              // Number of columns visible

	// Settings
	enablePlusOne    bool
	shiftLeft        bool // true = shift left, false = shift right
	enableCarry      bool
	showCarryDots    bool
	showColumnLine   bool
	hideRedRows      bool
	showTogglePreview bool

	// Key repeat
	keyRepeatFrame int

	// Toggle preview cache
	togglePreviewCache map[string]map[string]bool // "row-col" -> true if would change
}

// NewCrystalsChapter creates a new Crystals chapter
func NewCrystalsChapter() *CrystalsChapter {
	// Calculate visible rows based on screen height
	// Title (20) + settings (20) + grid start (60) + instructions (30) = 130px overhead
	// Cell size is 8px
	cellSize := 8
	availableHeight := screenHeight - 60 - 30 - cellSize // gridStartY to instructions, minus one cell for spacing
	visibleRows := availableHeight / cellSize
	if visibleRows < 1 {
		visibleRows = 1
	}

	c := &CrystalsChapter{
		rows:              []*SparseRow{NewSparseRow([]int{0})}, // Start with one bit at position 0
		carries:           []map[int]string{{}},
		currentRow:        0,
		currentCol:        0,
		scrollOffset:      0,
		horizontalOffset:  -75, // Center around 0
		visibleRows:       visibleRows,
		visibleCols:       150,
		enablePlusOne:     true,
		shiftLeft:         true,
		enableCarry:       true,
		showCarryDots:     true,
		showColumnLine:    true,
		hideRedRows:       false,
		showTogglePreview: false,
		togglePreviewCache: make(map[string]map[string]bool),
	}
	c.generateSequence()
	return c
}

// generateSequence generates the Collatz sequence
func (c *CrystalsChapter) generateSequence() {
	// Clear all rows after the first one
	c.rows = []*SparseRow{c.rows[0].Clone()}
	c.carries = []map[int]string{{}}
	// Clear toggle preview cache when sequence changes
	c.togglePreviewCache = make(map[string]map[string]bool)

	// Generate initial sequence
	initialSteps := c.visibleRows + 100
	c.generateSequenceSteps(initialSteps)
}

// generateSequenceSteps generates sequence steps
func (c *CrystalsChapter) generateSequenceSteps(maxSteps int) {
	startLength := len(c.rows)
	currentRowIndex := 0

	for i := 0; i < maxSteps; i++ {
		if currentRowIndex >= len(c.rows) {
			break
		}

		currentRow := c.rows[currentRowIndex].Clone()

		var shiftedRow *SparseRow
		if c.shiftLeft {
			shiftedRow = currentRow.ShiftLeft(1)
		} else {
			shiftedRow = currentRow.ShiftRight(1)
		}

		var plusOneRow *SparseRow
		if c.enablePlusOne && !shiftedRow.IsZero() {
			rightmostPos := currentRow.GetRightmostPosition()
			if !c.shiftLeft {
				rightmostPos++
			}
			plusOneRow = CreatePlusOne(rightmostPos)
		}

		var nextRow *SparseRow
		var nextCarries map[int]string

		if c.enableCarry {
			result := AddWithCarries(currentRow, shiftedRow, plusOneRow)
			nextRow = result.Result
			nextCarries = result.Carries
		} else {
			tempRow := AddSparseRowsNoCarry(currentRow, shiftedRow)
			if c.enablePlusOne && plusOneRow != nil {
				tempRow = AddSparseRowsNoCarry(tempRow, plusOneRow)
			}
			nextRow = tempRow
			nextCarries = make(map[int]string)
		}

		c.rows = append(c.rows, shiftedRow)
		c.carries = append(c.carries, make(map[int]string))

		if c.enablePlusOne && plusOneRow != nil {
			c.rows = append(c.rows, plusOneRow)
			c.carries = append(c.carries, make(map[int]string))
		}

		c.rows = append(c.rows, nextRow)
		c.carries = append(c.carries, nextCarries)

		currentRowIndex = len(c.rows) - 1
	}

	// Ensure we generated new rows
	if len(c.rows) <= startLength {
		return
	}
}

// getRowType determines if a row is green (main sequence) or red (intermediate)
func (c *CrystalsChapter) getRowType(rowIndex int) string {
	mod := 2
	if c.enablePlusOne {
		mod = 3
	}
	if rowIndex%mod == 0 {
		return "green"
	}
	return "red"
}

// calculateKAndGValues calculates k and g values for a green row
func (c *CrystalsChapter) calculateKAndGValues(greenRowIndex int) (k, g int) {
	currentRow := c.rows[greenRowIndex]
	if currentRow == nil || currentRow.IsZero() {
		return 0, 0
	}

	// Find the previous green row
	mod := 2
	if c.enablePlusOne {
		mod = 3
	}
	var previousGreenRow *SparseRow
	for i := greenRowIndex - 1; i >= 0; i-- {
		if i%mod == 0 {
			previousGreenRow = c.rows[i]
			break
		}
	}

	if previousGreenRow == nil || previousGreenRow.IsZero() {
		return 0, 0
	}

	currentMax := currentRow.GetLeftmostPosition()
	currentMin := currentRow.GetRightmostPosition()
	previousMax := previousGreenRow.GetLeftmostPosition()
	previousMin := previousGreenRow.GetRightmostPosition()

	k = int(math.Abs(float64(currentMin - previousMin)))
	g = int(math.Abs(float64(currentMax - previousMax)))

	return k, g
}

// getMostSignificantDigitColumn returns the MSD column of the initial sequence
func (c *CrystalsChapter) getMostSignificantDigitColumn() *int {
	if len(c.rows) == 0 || c.rows[0] == nil || c.rows[0].IsZero() {
		return nil
	}
	msd := c.rows[0].GetLeftmostPosition()
	return &msd
}

// getTogglePreviewChanges calculates which cells would change if the current cell were toggled
func (c *CrystalsChapter) getTogglePreviewChanges() map[string]bool {
	if !c.showTogglePreview {
		return make(map[string]bool)
	}

	// Create cache key
	cacheKey := strconv.Itoa(c.currentRow) + "-" + strconv.Itoa(c.currentCol)
	
	// Check cache (simple implementation - could add time-based caching later)
	if cached, ok := c.togglePreviewCache[cacheKey]; ok {
		return cached
	}

	// Store original state
	originalRows := make([]*SparseRow, len(c.rows))
	for i, row := range c.rows {
		if row != nil {
			originalRows[i] = row.Clone()
		} else {
			originalRows[i] = NewSparseRow([]int{})
		}
	}
	originalCarries := make([]map[int]string, len(c.carries))
	for i, carry := range c.carries {
		originalCarries[i] = make(map[int]string)
		for k, v := range carry {
			originalCarries[i][k] = v
		}
	}

	// Make the invisible change to the first row (the editable row)
	if c.rows[0] == nil {
		c.rows[0] = NewSparseRow([]int{})
	}
	c.rows[0].FlipBit(c.currentCol)

	// Clear and regenerate the sequence
	c.rows = []*SparseRow{c.rows[0].Clone()}
	c.carries = []map[int]string{{}}
	c.generateSequenceSteps(len(originalRows) + 50)

	// Compare and find differences
	changedCells := make(map[string]bool)
	maxRows := len(c.rows)
	if len(originalRows) < maxRows {
		maxRows = len(originalRows)
	}

	for i := 0; i < maxRows; i++ {
		originalRow := originalRows[i]
		newRow := c.rows[i]
		
		if originalRow != nil && newRow != nil {
			// Find positions that are different
			originalPositions := make(map[int]bool)
			for _, pos := range originalRow.positions {
				originalPositions[pos] = true
			}
			newPositions := make(map[int]bool)
			for _, pos := range newRow.positions {
				newPositions[pos] = true
			}

			// Check for removed positions
			for pos := range originalPositions {
				if !newPositions[pos] {
					cellKey := strconv.Itoa(i) + "-" + strconv.Itoa(pos)
					changedCells[cellKey] = true
				}
			}
			// Check for added positions
			for pos := range newPositions {
				if !originalPositions[pos] {
					cellKey := strconv.Itoa(i) + "-" + strconv.Itoa(pos)
					changedCells[cellKey] = true
				}
			}
		}
	}

	// Restore the original state
	c.rows = originalRows
	c.carries = originalCarries

	// Cache the result
	c.togglePreviewCache[cacheKey] = changedCells
	return changedCells
}

// Update updates the Crystals chapter
func (c *CrystalsChapter) Update() error {
	c.escConsumed = false // Reset at start of frame - ESC is not consumed by this chapter
	c.keyRepeatFrame++

	// Handle arrow keys for scrolling
	keyRepeatDelay := 5
	canMove := c.keyRepeatFrame%keyRepeatDelay == 0

	if inpututil.IsKeyJustPressed(ebiten.KeyArrowUp) || (ebiten.IsKeyPressed(ebiten.KeyArrowUp) && canMove) {
		c.scrollGrid(-1)
	}
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowDown) || (ebiten.IsKeyPressed(ebiten.KeyArrowDown) && canMove) {
		c.scrollGrid(1)
	}
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowLeft) || (ebiten.IsKeyPressed(ebiten.KeyArrowLeft) && canMove) {
		c.scrollHorizontal(-1)
	}
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowRight) || (ebiten.IsKeyPressed(ebiten.KeyArrowRight) && canMove) {
		c.scrollHorizontal(1)
	}

	// Handle Shift+arrow keys for cursor movement
	shiftPressed := ebiten.IsKeyPressed(ebiten.KeyShiftLeft) || ebiten.IsKeyPressed(ebiten.KeyShiftRight)
	if shiftPressed {
		if inpututil.IsKeyJustPressed(ebiten.KeyArrowLeft) || (ebiten.IsKeyPressed(ebiten.KeyArrowLeft) && canMove) {
			c.currentCol--
			c.horizontalOffset = c.currentCol - c.visibleCols/2
		}
		if inpututil.IsKeyJustPressed(ebiten.KeyArrowRight) || (ebiten.IsKeyPressed(ebiten.KeyArrowRight) && canMove) {
			c.currentCol++
			c.horizontalOffset = c.currentCol - c.visibleCols/2
		}
	}

	// Handle space to flip bit
	if inpututil.IsKeyJustPressed(ebiten.KeySpace) {
		c.flipBit()
	}

	// Handle settings toggles
	if inpututil.IsKeyJustPressed(ebiten.KeyP) {
		c.enablePlusOne = !c.enablePlusOne
		c.generateSequence()
	}
	if inpututil.IsKeyJustPressed(ebiten.KeyS) {
		c.shiftLeft = !c.shiftLeft
		c.generateSequence()
	}
	if inpututil.IsKeyJustPressed(ebiten.KeyC) {
		c.enableCarry = !c.enableCarry
		c.generateSequence()
	}
	if inpututil.IsKeyJustPressed(ebiten.KeyD) {
		c.showCarryDots = !c.showCarryDots
	}
	if inpututil.IsKeyJustPressed(ebiten.KeyL) {
		c.showColumnLine = !c.showColumnLine
	}
	if inpututil.IsKeyJustPressed(ebiten.KeyH) {
		c.hideRedRows = !c.hideRedRows
	}
	if inpututil.IsKeyJustPressed(ebiten.KeyT) {
		c.showTogglePreview = !c.showTogglePreview
		// Clear cache when toggling preview on/off
		c.togglePreviewCache = make(map[string]map[string]bool)
	}

	// Handle reset
	if inpututil.IsKeyJustPressed(ebiten.KeyR) {
		c.rows[0] = NewSparseRow([]int{})
		c.currentCol = 0
		c.horizontalOffset = -c.visibleCols / 2
		c.scrollOffset = 0
		c.generateSequence()
	}

	return nil
}

// scrollGrid scrolls the grid vertically
func (c *CrystalsChapter) scrollGrid(direction int) {
	c.scrollOffset += direction
	if c.scrollOffset < 0 {
		c.scrollOffset = 0
	}

	currentEndRow := c.scrollOffset + c.visibleRows
	bufferSize := 50

	if currentEndRow >= len(c.rows)-bufferSize {
		moreSteps := c.visibleRows + bufferSize
		c.generateSequenceSteps(moreSteps)
	}
}

// scrollHorizontal scrolls the grid horizontally
func (c *CrystalsChapter) scrollHorizontal(direction int) {
	c.horizontalOffset += direction
}

// flipBit flips the bit at the current position
func (c *CrystalsChapter) flipBit() {
	if c.currentRow >= len(c.rows) {
		return
	}
	if c.rows[c.currentRow] == nil {
		c.rows[c.currentRow] = NewSparseRow([]int{})
	}

	c.rows[c.currentRow].FlipBit(c.currentCol)
	c.generateSequence()
}

// Draw draws the Crystals chapter
func (c *CrystalsChapter) Draw(screen *ebiten.Image) {
	// Fill background
	screen.Fill(color.RGBA{15, 15, 15, 255})

	// Draw title
	titleText := "Crystals - Collatz Binary Visualization"
	titleBounds := text.BoundString(basicfont.Face7x13, titleText)
	titleX := (screenWidth - titleBounds.Dx()) / 2
	titleY := 20
	text.Draw(screen, titleText, basicfont.Face7x13, titleX, titleY, color.White)

	// Draw settings
	settingsY := 40
	settingsText := "P: +1 | S: Shift | C: Carry | D: Dots | L: Line | H: Hide Red | T: Preview | R: Reset"
	text.Draw(screen, settingsText, basicfont.Face7x13, 10, settingsY, color.Gray{Y: 150})

	// Grid parameters
	cellSize := 8
	rowLabelWidth := 120
	gridStartX := rowLabelWidth + 10
	gridStartY := 60

	startRow := c.scrollOffset
	endRow := startRow + c.visibleRows
	if endRow > len(c.rows) {
		endRow = len(c.rows)
	}
	startCol := c.horizontalOffset
	endCol := startCol + c.visibleCols

	// Calculate green row numbers
	mod := 2
	if c.enablePlusOne {
		mod = 3
	}
	greenRowNumber := 0
	for i := 0; i < startRow; i++ {
		if i%mod == 0 {
			greenRowNumber++
		}
	}

	// Draw rows
	visualRowIndex := 0
	for i := startRow; i < endRow; i++ {
		if i >= len(c.rows) {
			break
		}

		rowType := c.getRowType(i)

		// Skip red rows if hideRedRows is enabled
		if c.hideRedRows && rowType == "red" {
			continue
		}

		if rowType == "green" {
			greenRowNumber++
		}

		rowY := gridStartY + visualRowIndex*cellSize

		// Draw row label
		if rowType == "green" {
			// Draw row number
			rowNumText := strconv.Itoa(greenRowNumber)
			text.Draw(screen, rowNumText, basicfont.Face7x13, 10, rowY+cellSize-2, color.White)

			// Draw k, g, n values
			if c.rows[i] != nil {
				k, g := c.calculateKAndGValues(i)
				n := c.rows[i].ToDecimal()
				valuesText := "g:" + strconv.Itoa(g) + " k:" + strconv.Itoa(k) + " n:" + strconv.Itoa(n)
				text.Draw(screen, valuesText, basicfont.Face7x13, 40, rowY+cellSize-2, color.Gray{Y: 200})
			}
		}

		// Draw cells
		for j := startCol; j < endCol; j++ {
			cellX := gridStartX + (j-startCol)*cellSize

			row := c.rows[i]
			if row == nil {
				row = NewSparseRow([]int{})
			}

			hasOne := row.HasBitAt(j)

			// Determine cell color
			var cellColor color.Color
			if !hasOne {
				cellColor = color.RGBA{224, 224, 224, 255} // Grey
			} else if rowType == "green" {
				cellColor = color.RGBA{0, 150, 0, 255} // Darker, more saturated green
			} else {
				cellColor = color.RGBA{200, 0, 0, 255} // Darker, more saturated red
			}

			// Draw cell
			for dy := 0; dy < cellSize-1; dy++ {
				for dx := 0; dx < cellSize-1; dx++ {
					screen.Set(cellX+dx, rowY+dy, cellColor)
				}
			}

			// Draw carry dots for green rows
			if c.showCarryDots && c.enableCarry && rowType == "green" && i < len(c.carries) {
				carryType, hasCarry := c.carries[i][j]
				if hasCarry {
					dotColor := color.RGBA{112, 128, 144, 255} // Slate grey
					if carryType == "plus-one" {
						dotColor = color.RGBA{112, 128, 144, 255}
					} else if carryType == "later" {
						dotColor = color.RGBA{128, 128, 0, 255} // Olive
					}
					// Draw a larger dot (3x3 pixels instead of 1 pixel)
					dotSize := 3
					dotX := cellX + cellSize - dotSize - 1
					dotY := rowY + 1
					for dy := 0; dy < dotSize; dy++ {
						for dx := 0; dx < dotSize; dx++ {
							screen.Set(dotX+dx, dotY+dy, dotColor)
						}
					}
				}
			}

			// Draw MSD highlight
			msdCol := c.getMostSignificantDigitColumn()
			if msdCol != nil && j == *msdCol {
				// Draw orange border
				for dx := 0; dx < cellSize-1; dx++ {
					screen.Set(cellX+dx, rowY, color.RGBA{255, 140, 0, 255})
					screen.Set(cellX+dx, rowY+cellSize-2, color.RGBA{255, 140, 0, 255})
				}
				for dy := 0; dy < cellSize-1; dy++ {
					screen.Set(cellX, rowY+dy, color.RGBA{255, 140, 0, 255})
					screen.Set(cellX+cellSize-2, rowY+dy, color.RGBA{255, 140, 0, 255})
				}
			}

			// Draw cursor
			if i == c.currentRow && j == c.currentCol {
				// Draw orange outline
				for dx := 0; dx < cellSize-1; dx++ {
					screen.Set(cellX+dx, rowY, color.RGBA{255, 140, 0, 255})
					screen.Set(cellX+dx, rowY+cellSize-2, color.RGBA{255, 140, 0, 255})
				}
				for dy := 0; dy < cellSize-1; dy++ {
					screen.Set(cellX, rowY+dy, color.RGBA{255, 140, 0, 255})
					screen.Set(cellX+cellSize-2, rowY+dy, color.RGBA{255, 140, 0, 255})
				}
			}

			// Draw selected column line
			if c.showColumnLine && j == c.currentCol {
				// Draw dotted line on right side with more saturated orange
				if (visualRowIndex)%2 == 0 {
					screen.Set(cellX+cellSize-1, rowY, color.RGBA{255, 165, 0, 255}) // More saturated orange
					screen.Set(cellX+cellSize-1, rowY+cellSize-2, color.RGBA{255, 165, 0, 255})
				}
			}

			// Draw toggle preview highlight
			if c.showTogglePreview {
				changedCells := c.getTogglePreviewChanges()
				cellKey := strconv.Itoa(i) + "-" + strconv.Itoa(j)
				if changedCells[cellKey] {
					// Draw more saturated cyan outline
					for dx := 0; dx < cellSize-1; dx++ {
						screen.Set(cellX+dx, rowY, color.RGBA{0, 191, 255, 255}) // Deep sky blue
						screen.Set(cellX+dx, rowY+cellSize-2, color.RGBA{0, 191, 255, 255})
					}
					for dy := 0; dy < cellSize-1; dy++ {
						screen.Set(cellX, rowY+dy, color.RGBA{0, 191, 255, 255})
						screen.Set(cellX+cellSize-2, rowY+dy, color.RGBA{0, 191, 255, 255})
					}
				}
			}
		}

		visualRowIndex++
	}

	// Draw instructions
	instructions := "Arrow Keys: Scroll | Shift+Arrow: Move Cursor | Space: Flip Bit | ESC: Return"
	instBounds := text.BoundString(basicfont.Face7x13, instructions)
	instX := (screenWidth - instBounds.Dx()) / 2
	instY := screenHeight - 30
	text.Draw(screen, instructions, basicfont.Face7x13, instX, instY, color.Gray{Y: 150})
}

// WasEscConsumed returns whether ESC was consumed by a dialog this frame
func (c *CrystalsChapter) WasEscConsumed() bool {
	return c.escConsumed
}
