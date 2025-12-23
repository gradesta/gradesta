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

// GrammarForFiniteSentencesChapter implements the A Grammar for Finite Sentences chapter
type GrammarForFiniteSentencesChapter struct {
	// Row values (exponents for the algorithm)
	// Each row can have a value >= 0, or -1 if not set yet
	rowValues []int
	
	// Currently selected row (0-indexed)
	selectedRow int
	
	// Scroll offset for displaying rows
	scrollOffset int
	
	// Cached staircase calculations for each row
	cachedSteps map[int][]StairStep // Maps row index to steps
	cachedCoef  float64             // Coefficient used for cached steps
	
	escConsumed bool // Whether ESC was consumed by a dialog this frame
}

// NewGrammarForFiniteSentencesChapter creates a new A Grammar for Finite Sentences chapter
func NewGrammarForFiniteSentencesChapter() *GrammarForFiniteSentencesChapter {
	// Start with one row set to 0 (which produces index 1)
	return &GrammarForFiniteSentencesChapter{
		rowValues:    []int{0},
		selectedRow:  0,
		cachedSteps:  make(map[int][]StairStep),
		cachedCoef:   3.0, // Default coefficient
	}
}

// calculateIndex calculates the index using the grammar algorithm
// For values [v1, v2, v3, ..., vn], the formula is:
// result = (2^v1 - 1) / 3
// result = (result * 2^v2 - 1) / 3
// result = (result * 2^v3 - 1) / 3
// ... and so on
func (g *GrammarForFiniteSentencesChapter) calculateIndex() *big.Int {
	if len(g.rowValues) == 0 {
		return big.NewInt(1)
	}
	
	// Start with first value
	firstValue := g.rowValues[0]
	if firstValue < 0 {
		// If first value is not set, return 1
		return big.NewInt(1)
	}
	
	// Calculate (2^firstValue - 1) / 3
	three := big.NewInt(3)
	one := big.NewInt(1)
	
	// 2^firstValue
	powerOf2 := new(big.Int).Lsh(one, uint(firstValue)) // 1 << firstValue
	// 2^firstValue - 1
	powerOf2Minus1 := new(big.Int).Sub(powerOf2, one)
	// (2^firstValue - 1) / 3
	result := new(big.Int).Div(powerOf2Minus1, three)
	
	// Check if division is exact
	remainder := new(big.Int).Mod(powerOf2Minus1, three)
	if remainder.Sign() != 0 {
		// Not divisible by 3, return 0 (invalid)
		return big.NewInt(0)
	}
	
	// Continue with remaining values
	for i := 1; i < len(g.rowValues); i++ {
		value := g.rowValues[i]
		if value < 0 {
			// This value is not set, stop here
			break
		}
		
		// Multiply by 2^value
		powerOf2 := new(big.Int).Lsh(one, uint(value)) // 1 << value
		result.Mul(result, powerOf2)
		
		// Subtract 1
		result.Sub(result, one)
		
		// Divide by 3
		remainder := new(big.Int).Mod(result, three)
		if remainder.Sign() != 0 {
			// Not divisible by 3, return 0 (invalid)
			return big.NewInt(0)
		}
		result.Div(result, three)
	}
	
	return result
}

// findValidValues finds valid values for a given row index
// A value is valid if setting it produces a whole number result
func (g *GrammarForFiniteSentencesChapter) findValidValues(rowIndex int) []int {
	validValues := []int{}
	
	// Try values from 0 to some reasonable maximum (e.g., 50)
	maxValue := 50
	for v := 0; v <= maxValue; v++ {
		// Temporarily set this value
		oldValue := -1
		if rowIndex < len(g.rowValues) {
			oldValue = g.rowValues[rowIndex]
			g.rowValues[rowIndex] = v
		} else {
			// Need to expand the array
			for len(g.rowValues) <= rowIndex {
				g.rowValues = append(g.rowValues, -1)
			}
			g.rowValues[rowIndex] = v
		}
		
		// Check if this produces a valid (whole number) result
		result := g.calculateIndex()
		if result.Sign() > 0 {
			// Valid value
			validValues = append(validValues, v)
		}
		
		// Restore old value
		if oldValue >= 0 {
			g.rowValues[rowIndex] = oldValue
		} else {
			// Remove the value we added
			g.rowValues[rowIndex] = -1
		}
	}
	
	return validValues
}

// getCurrentValueIndex returns the index of the current value in the valid values list
func (g *GrammarForFiniteSentencesChapter) getCurrentValueIndex(rowIndex int) int {
	if rowIndex >= len(g.rowValues) || g.rowValues[rowIndex] < 0 {
		return -1
	}
	
	validValues := g.findValidValues(rowIndex)
	currentValue := g.rowValues[rowIndex]
	
	for i, v := range validValues {
		if v == currentValue {
			return i
		}
	}
	
	return -1
}

// Update updates the A Grammar for Finite Sentences chapter
func (g *GrammarForFiniteSentencesChapter) Update() error {
	g.escConsumed = false // Reset at start of frame
	
	// Handle up arrow: move to previous row
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowUp) || inpututil.IsKeyJustPressed(ebiten.KeyW) {
		if g.selectedRow > 0 {
			g.selectedRow--
			// Adjust scroll if needed
			if g.selectedRow < g.scrollOffset {
				g.scrollOffset = g.selectedRow
			}
		}
	}
	
	// Handle down arrow: move to next row
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowDown) || inpututil.IsKeyJustPressed(ebiten.KeyS) {
		g.selectedRow++
		// Expand array if needed
		for len(g.rowValues) <= g.selectedRow {
			g.rowValues = append(g.rowValues, -1)
		}
		// Adjust scroll if needed
		visibleRows := (screenHeight - 200) / 15 // Approximate visible rows
		if g.selectedRow >= g.scrollOffset+visibleRows {
			g.scrollOffset = g.selectedRow - visibleRows + 1
		}
	}
	
	// Handle left arrow: cycle to previous valid value
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowRight) || inpututil.IsKeyJustPressed(ebiten.KeyD) {
		validValues := g.findValidValues(g.selectedRow)
		if len(validValues) > 0 {
			currentIndex := g.getCurrentValueIndex(g.selectedRow)
			if currentIndex < 0 {
				// No current value, set to first valid value
				if len(g.rowValues) <= g.selectedRow {
					for len(g.rowValues) <= g.selectedRow {
						g.rowValues = append(g.rowValues, -1)
					}
				}
				g.rowValues[g.selectedRow] = validValues[0]
			} else {
				// Cycle to next value (wrap around)
				newIndex := (currentIndex + 1) % len(validValues)
				g.rowValues[g.selectedRow] = validValues[newIndex]
			}
			// Invalidate cache for this row and all subsequent rows
			g.invalidateCacheFromRow(g.selectedRow)
		}
	}
	
	// Handle left arrow: also invalidate cache
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowLeft) || inpututil.IsKeyJustPressed(ebiten.KeyA) {
		validValues := g.findValidValues(g.selectedRow)
		if len(validValues) > 0 {
			currentIndex := g.getCurrentValueIndex(g.selectedRow)
			if currentIndex < 0 {
				// No current value, set to first valid value
				if len(g.rowValues) <= g.selectedRow {
					for len(g.rowValues) <= g.selectedRow {
						g.rowValues = append(g.rowValues, -1)
					}
				}
				g.rowValues[g.selectedRow] = validValues[0]
			} else {
				// Cycle to previous value (wrap around)
				newIndex := (currentIndex - 1 + len(validValues)) % len(validValues)
				g.rowValues[g.selectedRow] = validValues[newIndex]
			}
			// Invalidate cache for this row and all subsequent rows
			g.invalidateCacheFromRow(g.selectedRow)
		}
	}
	
	return nil
}

// Draw draws the A Grammar for Finite Sentences chapter
func (g *GrammarForFiniteSentencesChapter) Draw(screen *ebiten.Image) {
	// Fill background
	screen.Fill(color.RGBA{20, 20, 30, 255})
	
	// Draw title
	titleText := "A Grammar for Finite Sentences"
	titleBounds := text.BoundString(basicfont.Face7x13, titleText)
	titleX := (screenWidth - titleBounds.Dx()) / 2
	titleY := 20
	text.Draw(screen, titleText, basicfont.Face7x13, titleX, titleY, color.White)
	
	// Calculate the index
	index := g.calculateIndex()
	indexText := "Index: " + formatNumber(index.String())
	indexBounds := text.BoundString(basicfont.Face7x13, indexText)
	indexX := (screenWidth - indexBounds.Dx()) / 2
	indexY := 40
	var textColor color.Color = color.White
	if index.Sign() == 0 {
		indexText = "Index: INVALID"
		textColor = color.RGBA{255, 100, 100, 255} // Red for invalid
	}
	text.Draw(screen, indexText, basicfont.Face7x13, indexX, indexY, textColor)
	
	// Draw table header (grammar table on left)
	headerY := 70
	lineHeight := 15
	colRowX := 10
	colValueX := 60
	colIndexX := 200
	
	text.Draw(screen, "Row", basicfont.Face7x13, colRowX, headerY, color.RGBA{200, 200, 255, 255})
	text.Draw(screen, "Value", basicfont.Face7x13, colValueX, headerY, color.RGBA{200, 200, 255, 255})
	text.Draw(screen, "Index", basicfont.Face7x13, colIndexX, headerY, color.RGBA{200, 200, 255, 255})
	
	// Draw rows
	visibleRows := (screenHeight - headerY - 50) / lineHeight
	startRow := g.scrollOffset
	endRow := startRow + visibleRows
	if endRow > len(g.rowValues) {
		endRow = len(g.rowValues)
	}
	
	// Always show at least a few rows ahead
	if endRow < len(g.rowValues) {
		endRow++
	}
	
	for i := startRow; i < endRow; i++ {
		rowY := headerY + lineHeight*(i-startRow+1)
		
		// Row number
		rowText := strconv.Itoa(i + 1)
		text.Draw(screen, rowText, basicfont.Face7x13, colRowX, rowY, color.White)
		
		// Value
		var valueText string
		var valueColor color.Color
		if i < len(g.rowValues) && g.rowValues[i] >= 0 {
			valueText = strconv.Itoa(g.rowValues[i])
			if i == g.selectedRow {
				valueColor = color.RGBA{255, 255, 100, 255} // Yellow for selected
			} else {
				valueColor = color.White
			}
		} else {
			valueText = "-"
			if i == g.selectedRow {
				valueColor = color.RGBA{150, 150, 150, 255} // Gray for selected but unset
			} else {
				valueColor = color.Color(color.Gray{Y: 100})
			}
		}
		text.Draw(screen, valueText, basicfont.Face7x13, colValueX, rowY, valueColor)
		
		// Calculate index up to this row
		// Temporarily calculate with values up to this row
		partialIndex := g.calculatePartialIndex(i + 1)
		if partialIndex.Sign() > 0 {
			indexText := formatNumber(partialIndex.String())
			text.Draw(screen, indexText, basicfont.Face7x13, colIndexX, rowY, color.RGBA{100, 255, 255, 255}) // Cyan
		} else {
			text.Draw(screen, "INVALID", basicfont.Face7x13, colIndexX, rowY, color.RGBA{255, 100, 100, 255}) // Red
		}
	}
	
	// Draw the full Collatz steps table for the selected row on the right side
	if g.selectedRow < len(g.rowValues) {
		selectedIndex := g.calculatePartialIndex(g.selectedRow + 1)
		if selectedIndex.Sign() > 0 {
			steps := g.getStepsForIndex(selectedIndex, g.selectedRow+1)
			
			// Draw steps table on the right side of the screen
			stepsTableX := 350
			stepsTableY := headerY
			
			// Draw header for steps table
			colStepX := stepsTableX
			colStepIndexX := stepsTableX + 50
			colStepYX := stepsTableX + 120
			colStepKX := stepsTableX + 190
			colStepDestX := stepsTableX + 230
			
			text.Draw(screen, "Step", basicfont.Face7x13, colStepX, stepsTableY, color.RGBA{200, 200, 255, 255})
			text.Draw(screen, "Index", basicfont.Face7x13, colStepIndexX, stepsTableY, color.RGBA{200, 200, 255, 255})
			text.Draw(screen, "Y", basicfont.Face7x13, colStepYX, stepsTableY, color.RGBA{200, 200, 255, 255})
			text.Draw(screen, "K", basicfont.Face7x13, colStepKX, stepsTableY, color.RGBA{200, 200, 255, 255})
			text.Draw(screen, "Dest", basicfont.Face7x13, colStepDestX, stepsTableY, color.RGBA{200, 200, 255, 255})
			
			// Draw starting point (row 0) - always use selectedIndex
			coefBig := big.NewInt(int64(g.cachedCoef))
			rowY := stepsTableY + lineHeight + 2
			startYValueBig := new(big.Int).Mul(coefBig, selectedIndex)
			startYValueBig.Add(startYValueBig, big.NewInt(1))
			startK := findLargestPowerOf2Big(startYValueBig)
			
			// Draw step number
			text.Draw(screen, "0", basicfont.Face7x13, colStepX, rowY, color.RGBA{255, 255, 100, 255}) // Yellow for start
			
			// Draw index (use selectedIndex)
			indexText := formatNumber(selectedIndex.String())
			text.Draw(screen, indexText, basicfont.Face7x13, colStepIndexX, rowY, color.RGBA{255, 255, 100, 255})
			
			// Draw Y value
			yText := formatNumber(startYValueBig.String())
			text.Draw(screen, yText, basicfont.Face7x13, colStepYX, rowY, color.RGBA{255, 255, 100, 255})
			
			// Draw K value
			kText := strconv.Itoa(startK)
			text.Draw(screen, kText, basicfont.Face7x13, colStepKX, rowY, color.RGBA{255, 255, 100, 255})
			
			// Draw destination (if K > 0)
			if startK > 0 {
				powerOf2Big := new(big.Int).Lsh(big.NewInt(1), uint(startK))
				destinationIndexBig := new(big.Int).Div(startYValueBig, powerOf2Big)
				destText := formatNumber(destinationIndexBig.String())
				text.Draw(screen, destText, basicfont.Face7x13, colStepDestX, rowY, color.RGBA{100, 255, 255, 255}) // Cyan
			} else {
				text.Draw(screen, "-", basicfont.Face7x13, colStepDestX, rowY, color.Gray{Y: 100})
			}
			
			// Draw step rows (all steps from the array)
			// Skip the first step if its StartIndex matches selectedIndex (already shown as row 0)
			stepOffset := 0
			if len(steps) > 0 && steps[0].StartIndex.Cmp(selectedIndex) == 0 {
				stepOffset = 1 // Skip first step, it's already shown as row 0
			}
			
			stepNum := 1
			for i := stepOffset; i < len(steps); i++ {
				step := steps[i]
				rowY = stepsTableY + lineHeight*(stepNum+1) + 2
				if rowY >= screenHeight-50 {
					break // Stop if we've run out of screen space
				}
				
				// Calculate Y value from StartIndex
				stepYValueBig := new(big.Int).Mul(coefBig, step.StartIndex)
				stepYValueBig.Add(stepYValueBig, big.NewInt(1))
				
				// Draw step number
				stepText := strconv.Itoa(stepNum)
				text.Draw(screen, stepText, basicfont.Face7x13, colStepX, rowY, color.White)
				
				// Draw index
				indexText := formatNumber(step.StartIndex.String())
				text.Draw(screen, indexText, basicfont.Face7x13, colStepIndexX, rowY, color.White)
				
				// Draw Y value
				yText := formatNumber(stepYValueBig.String())
				text.Draw(screen, yText, basicfont.Face7x13, colStepYX, rowY, color.White)
				
				// Draw K value
				kText := strconv.Itoa(step.K)
				text.Draw(screen, kText, basicfont.Face7x13, colStepKX, rowY, color.White)
				
				// Draw destination
				destText := formatNumber(step.DestinationIndex.String())
				text.Draw(screen, destText, basicfont.Face7x13, colStepDestX, rowY, color.RGBA{100, 255, 255, 255}) // Cyan
				
				stepNum++
			}
		}
	}
	
	// Draw instructions
	instructions := "UP/DOWN: select row | LEFT/RIGHT: cycle valid values | ESC: return"
	instBounds := text.BoundString(basicfont.Face7x13, instructions)
	instX := (screenWidth - instBounds.Dx()) / 2
	instY := screenHeight - 30
	text.Draw(screen, instructions, basicfont.Face7x13, instX, instY, color.Gray{Y: 150})
}

// calculatePartialIndex calculates the index using only the first n rows
func (g *GrammarForFiniteSentencesChapter) calculatePartialIndex(n int) *big.Int {
	if n == 0 {
		return big.NewInt(1)
	}
	
	if len(g.rowValues) == 0 {
		return big.NewInt(1)
	}
	
	// Start with first value
	firstValue := g.rowValues[0]
	if firstValue < 0 || n == 0 {
		return big.NewInt(1)
	}
	
	// Calculate (2^firstValue - 1) / 3
	three := big.NewInt(3)
	one := big.NewInt(1)
	
	// 2^firstValue
	powerOf2 := new(big.Int).Lsh(one, uint(firstValue))
	// 2^firstValue - 1
	powerOf2Minus1 := new(big.Int).Sub(powerOf2, one)
	// (2^firstValue - 1) / 3
	result := new(big.Int).Div(powerOf2Minus1, three)
	
	// Check if division is exact
	remainder := new(big.Int).Mod(powerOf2Minus1, three)
	if remainder.Sign() != 0 {
		return big.NewInt(0)
	}
	
	// Continue with remaining values up to n
	maxRows := n
	if maxRows > len(g.rowValues) {
		maxRows = len(g.rowValues)
	}
	
	for i := 1; i < maxRows; i++ {
		value := g.rowValues[i]
		if value < 0 {
			break
		}
		
		// Multiply by 2^value
		powerOf2 := new(big.Int).Lsh(one, uint(value))
		result.Mul(result, powerOf2)
		
		// Subtract 1
		result.Sub(result, one)
		
		// Divide by 3
		remainder := new(big.Int).Mod(result, three)
		if remainder.Sign() != 0 {
			return big.NewInt(0)
		}
		result.Div(result, three)
	}
	
	return result
}

// invalidateCacheFromRow invalidates the cache for the given row and all subsequent rows
func (g *GrammarForFiniteSentencesChapter) invalidateCacheFromRow(startRow int) {
	for rowNum := range g.cachedSteps {
		if rowNum >= startRow {
			delete(g.cachedSteps, rowNum)
		}
	}
}

// getStepsForIndex calculates the Collatz steps for a given index
func (g *GrammarForFiniteSentencesChapter) getStepsForIndex(index *big.Int, rowNum int) []StairStep {
	// Check cache first
	if cached, ok := g.cachedSteps[rowNum]; ok {
		// Verify cache is still valid - check if index matches
		// For now, we'll recalculate to ensure accuracy
		// Could optimize later by storing index with cache
		_ = cached
	}
	
	// Calculate steps using the Collatz algorithm
	steps, _, _ := CalculateStaircase(index, g.cachedCoef, 50, true)
	
	// Cache the result
	g.cachedSteps[rowNum] = steps
	
	return steps
}

// WasEscConsumed returns whether ESC was consumed by a dialog this frame
func (g *GrammarForFiniteSentencesChapter) WasEscConsumed() bool {
	return g.escConsumed
}
