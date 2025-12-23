package main

import (
	"crypto/sha256"
	"encoding/hex"
	"image/color"
	"log"
	"math"
	"math/big"
	"strconv"
	"strings"

	"github.com/hajimehoshi/ebiten/v2"
	"github.com/hajimehoshi/ebiten/v2/ebitenutil"
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
	
	// Global coefficient for the division (default 3)
	globalCoeficient float64
	
	// Cached staircase calculations for each row
	cachedSteps map[int][]StairStep // Maps row index to steps
	cachedCoef  float64             // Coefficient used for cached steps
	
	// Cached equation image
	equationImage *ebiten.Image
	equationHash  string // Hash of current equation to detect changes
	
	escConsumed bool // Whether ESC was consumed by a dialog this frame
}

// NewGrammarForFiniteSentencesChapter creates a new A Grammar for Finite Sentences chapter
func NewGrammarForFiniteSentencesChapter() *GrammarForFiniteSentencesChapter {
	// Start with one row set to 4
	return &GrammarForFiniteSentencesChapter{
		rowValues:        []int{4},
		selectedRow:      0,
		globalCoeficient: 3.0, // Default coefficient
		cachedSteps:      make(map[int][]StairStep),
		cachedCoef:       3.0, // Default coefficient
		equationHash:     "",
	}
}

// calculateIndex calculates the index using the grammar algorithm
// For values [v1, v2, v3, ..., vn] (where v1 is row 1, v2 is row 2, etc.):
// result = (2^v1 - 1) / 3
// result = (result * 2^v2 - 1) / 3
// result = (result * 2^v3 - 1) / 3
// ... and so on
// Note: Display shows rows in reverse order (row n at top, row 1 at bottom),
// but calculation processes from row 1 (array index 0) to row n (array index n-1)
func (g *GrammarForFiniteSentencesChapter) calculateIndex() *big.Int {
	if len(g.rowValues) == 0 {
		return big.NewInt(1)
	}
	
	// Start with first value (row 1, array index 0)
	firstValue := g.rowValues[0]
	if firstValue < 0 {
		// If first value is not set, return 1
		return big.NewInt(1)
	}
	
	// Calculate (2^firstValue - 1) / globalCoeficient
	coef := big.NewInt(int64(g.globalCoeficient))
	one := big.NewInt(1)
	
	// 2^firstValue
	powerOf2 := new(big.Int).Lsh(one, uint(firstValue)) // 1 << firstValue
	// 2^firstValue - 1
	powerOf2Minus1 := new(big.Int).Sub(powerOf2, one)
	// (2^firstValue - 1) / globalCoeficient
	result := new(big.Int).Div(powerOf2Minus1, coef)
	
	// Check if division is exact
	remainder := new(big.Int).Mod(powerOf2Minus1, coef)
	if remainder.Sign() != 0 {
		// Not divisible by globalCoeficient, return 0 (invalid)
		return big.NewInt(0)
	}
	
	// Continue with remaining values (from row 2 onwards, array index 1 onwards)
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
		
		// Divide by globalCoeficient
		remainder := new(big.Int).Mod(result, coef)
		if remainder.Sign() != 0 {
			// Not divisible by globalCoeficient, return 0 (invalid)
			return big.NewInt(0)
		}
		result.Div(result, coef)
	}
	
	return result
}

// findValidValues finds valid values for a given row index
// A value is valid if setting it produces a whole number result (calculated in reverse order)
// Tries values up to a reasonable limit
func (g *GrammarForFiniteSentencesChapter) findValidValues(rowIndex int) []int {
	validValues := []int{}
	
	// Try values from 0 up to a reasonable maximum
	// Use a higher limit than before, but still bounded to prevent infinite loops
	maxValue := 200
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
		
		// Check if this produces a valid (whole number) result (using reverse order calculation)
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
	
	// Handle 'r' key to reset - delete all rows and start fresh
	if inpututil.IsKeyJustPressed(ebiten.KeyR) {
		// Reset to initial state: one row with lowest valid value
		validValues := g.findValidValues(0)
		lowestValue := 0
		if len(validValues) > 0 {
			lowestValue = validValues[0] // First valid value is the lowest
		}
		g.rowValues = []int{lowestValue}
		g.selectedRow = 0
		g.scrollOffset = 0
		// Invalidate all caches
		g.cachedSteps = make(map[int][]StairStep)
		return nil
	}
	
	// Handle up arrow: add a new row at the top only if we're at the top (selectedRow == maxRow)
	maxRow := len(g.rowValues) - 1
	if maxRow < 0 {
		maxRow = 0
	}
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowUp) || inpututil.IsKeyJustPressed(ebiten.KeyW) {
		if g.selectedRow == maxRow {
			// Append a new row at the end (which displays at the top in reverse order)
			// First, expand the array to the new size to calculate valid values
			newRowIndex := len(g.rowValues)
			// Temporarily add a placeholder so findValidValues can work
			g.rowValues = append(g.rowValues, -1)
			// Now find valid values for this new row
			validValues := g.findValidValues(newRowIndex)
			lowestValue := 0
			if len(validValues) > 0 {
				lowestValue = validValues[0] // First valid value is the lowest
			}
			// Set the actual value (replacing the -1 placeholder)
			g.rowValues[newRowIndex] = lowestValue
			
			// Keep selectedRow at the new max (the new row we just added)
			g.selectedRow = len(g.rowValues) - 1
			
			// Reset scroll to top (in reverse order, top means scrollOffset = 0)
			g.scrollOffset = 0
			
			// Invalidate all caches
			g.cachedSteps = make(map[int][]StairStep)
			return nil
		}
		
		// Move to next row (higher row number, displayed higher on screen in reverse order)
		g.selectedRow++
		// Expand array if needed - but don't set to -1, set to lowest valid value
		for len(g.rowValues) <= g.selectedRow {
			// Temporarily add placeholder
			g.rowValues = append(g.rowValues, -1)
			// Find valid values for this new row
			rowIndex := len(g.rowValues) - 1
			validValues := g.findValidValues(rowIndex)
			lowestValue := 0
			if len(validValues) > 0 {
				lowestValue = validValues[0]
			}
			// Set the actual value (replacing the -1 placeholder)
			g.rowValues[rowIndex] = lowestValue
		}
		// No scrolling - always show from top
		g.scrollOffset = 0
	}
	
	// Handle down arrow: move to previous row (lower row number, displayed lower on screen in reverse order)
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowDown) || inpututil.IsKeyJustPressed(ebiten.KeyS) {
		if g.selectedRow > 0 {
			g.selectedRow--
		}
		// No scrolling - always show from top
		g.scrollOffset = 0
	}
	
	// Handle right arrow: increase value (no wrap around)
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowRight) || inpututil.IsKeyJustPressed(ebiten.KeyD) {
		validValues := g.findValidValues(g.selectedRow)
		if len(validValues) > 0 {
			currentIndex := g.getCurrentValueIndex(g.selectedRow)
			if currentIndex < 0 {
				// No current value, set to first valid value (0)
				if len(g.rowValues) <= g.selectedRow {
					for len(g.rowValues) <= g.selectedRow {
						g.rowValues = append(g.rowValues, -1)
					}
				}
				g.rowValues[g.selectedRow] = validValues[0]
			} else if currentIndex < len(validValues)-1 {
				// Increase to next value (no wrap around - stop at maximum)
				newIndex := currentIndex + 1
				g.rowValues[g.selectedRow] = validValues[newIndex]
				// Invalidate cache for this row and all subsequent rows
				g.invalidateCacheFromRow(g.selectedRow)
			}
			// If currentIndex == len(validValues)-1, do nothing (already at maximum)
		}
	}
	
	// Handle left arrow: decrease value (no wrap around), or delete row if at top with value 0
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowLeft) || inpututil.IsKeyJustPressed(ebiten.KeyA) {
		maxRow := len(g.rowValues) - 1
		if maxRow < 0 {
			maxRow = 0
		}
		
		// Check if we're at the top row (maxRow) and at the lowest valid value - delete the row
		if g.selectedRow == maxRow && g.selectedRow >= 0 && len(g.rowValues) > g.selectedRow {
			validValues := g.findValidValues(g.selectedRow)
			currentValue := g.rowValues[g.selectedRow]
			lowestValue := 0
			if len(validValues) > 0 {
				lowestValue = validValues[0] // First valid value is the lowest
			}
			if currentValue == lowestValue {
				// Delete the top row
				if len(g.rowValues) > 1 {
					// Remove the last element (top row in reverse display)
					g.rowValues = g.rowValues[:len(g.rowValues)-1]
					// Adjust selectedRow to the new top row
					g.selectedRow = len(g.rowValues) - 1
					if g.selectedRow < 0 {
						g.selectedRow = 0
					}
					// Reset scroll to top
					g.scrollOffset = 0
					// Invalidate all caches
					g.cachedSteps = make(map[int][]StairStep)
					return nil // Exit early after deletion
				} else {
					// Can't delete the last row, just reset it to lowest valid value
					resetValidValues := g.findValidValues(0)
					resetLowestValue := 0
					if len(resetValidValues) > 0 {
						resetLowestValue = resetValidValues[0]
					}
					g.rowValues[0] = resetLowestValue
					g.invalidateCacheFromRow(0)
					return nil
				}
			}
		}
		
		// Normal behavior: decrease value (no wrap around)
		validValues := g.findValidValues(g.selectedRow)
		if len(validValues) > 0 {
			currentIndex := g.getCurrentValueIndex(g.selectedRow)
			if currentIndex < 0 {
				// No current value, set to first valid value (which should be 0)
				if len(g.rowValues) <= g.selectedRow {
					for len(g.rowValues) <= g.selectedRow {
						g.rowValues = append(g.rowValues, -1)
					}
				}
				g.rowValues[g.selectedRow] = validValues[0]
			} else if currentIndex > 0 {
				// Decrease to previous value (no wrap around - stop at 0)
				newIndex := currentIndex - 1
				g.rowValues[g.selectedRow] = validValues[newIndex]
				// Invalidate cache for this row and all subsequent rows
				g.invalidateCacheFromRow(g.selectedRow)
			}
			// If currentIndex == 0, do nothing (already at minimum)
		}
	}
	
	// Handle 'c' key: cycle global coefficient
	if inpututil.IsKeyJustPressed(ebiten.KeyC) {
		g.cycleGlobalCoeficient()
		// Invalidate all caches when coefficient changes
		g.cachedSteps = make(map[int][]StairStep)
		g.equationHash = "" // Force equation re-render
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
	
	// Draw rows in REVERSE order (highest row number at top, row 1 at bottom)
	// No scrolling - always show all rows from top
	maxRow := len(g.rowValues) - 1
	if maxRow < 0 {
		maxRow = 0
	}
	
	// Display all rows from top (maxRow) down to row 0
	displayIndex := 0
	for i := maxRow; i >= 0; i-- {
		if i < 0 || i >= len(g.rowValues) {
			continue
		}
		rowY := headerY + lineHeight*(displayIndex+1)
		displayIndex++
		
		// Row number (displayed row number, 1-indexed from bottom)
		rowText := strconv.Itoa(i + 1)
		text.Draw(screen, rowText, basicfont.Face7x13, colRowX, rowY, color.White)
		
		// Value
		var valueText string
		var valueColor color.Color
		if g.rowValues[i] >= 0 {
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
		
		// Calculate index up to this row (from row 1 to this row)
		// n is the number of rows from the start (row 1 = index 0, row 2 = index 1, etc.)
		// Display shows row i+1, which corresponds to array index i
		// So we want to calculate using rows 1 through (i+1), which is (i+1) rows total
		n := i + 1
		partialIndex := g.calculatePartialIndexReverse(n)
		if partialIndex.Sign() > 0 {
			indexText := formatNumber(partialIndex.String())
			text.Draw(screen, indexText, basicfont.Face7x13, colIndexX, rowY, color.RGBA{100, 255, 255, 255}) // Cyan
		} else {
			text.Draw(screen, "INVALID", basicfont.Face7x13, colIndexX, rowY, color.RGBA{255, 100, 100, 255}) // Red
		}
	}
	
	// Draw the full Collatz steps table for the current index (from all rows) on the right side
	currentIndex := g.calculateIndex()
	if currentIndex.Sign() > 0 {
		// Calculate steps for the final index - use same parameters as spiral stairs
		steps, isUpwards, hitLimit := CalculateStaircase(currentIndex, g.globalCoeficient, 50, false)
		
		// Prepare data for the table - use same structure as spiral stairs
		tableData := CollatzTableData{
			Steps:      steps,
			StartIndex: new(big.Int).Set(currentIndex),
			Coefficient: g.globalCoeficient,
			IsUpwards:  isUpwards,
			HitLimit:   hitLimit,
			StopOnDirectionChange: false, // Continue even if direction changes, like spiral stairs
		}
		
		// Draw the table on the right side, aligned vertically with the editable table
		// The editable table header starts at headerY (70), so align the steps table there too
		DrawCollatzTableAt(screen, tableData, 350, headerY)
	}
	
	// Draw equation at the bottom
	g.drawEquation(screen)
	
	// Draw coefficient display
	coefText := "Coefficient: " + strconv.FormatFloat(g.globalCoeficient, 'f', -1, 64)
	coefX := 10
	coefY := 40
	text.Draw(screen, coefText, basicfont.Face7x13, coefX, coefY, color.RGBA{200, 200, 255, 255})
	
	// Draw instructions
	instructions := "UP/DOWN: select row | LEFT/RIGHT: cycle valid values | C: cycle coefficient | R: reset all to 0 | ESC: return"
	instBounds := text.BoundString(basicfont.Face7x13, instructions)
	instX := (screenWidth - instBounds.Dx()) / 2
	instY := screenHeight - 15
	text.Draw(screen, instructions, basicfont.Face7x13, instX, instY, color.Gray{Y: 150})
}

// drawEquation draws the grammar equation at the bottom of the screen using LaTeX/MathJax
func (g *GrammarForFiniteSentencesChapter) drawEquation(screen *ebiten.Image) {
	// Build equation parts
	validRows := g.getValidRows()
	if len(validRows) == 0 {
		return
	}
	
	// Check if equation has changed
	newHash := g.hashEquation()
	if newHash != g.equationHash || g.equationImage == nil {
		img, err := g.renderEquationImage(validRows)
		if err != nil {
			log.Fatalf("Failed to render equation: %v", err)
		}
		if img == nil {
			log.Fatalf("Equation rendering returned nil image")
		}
		g.equationImage = img
		g.equationHash = newHash
	}
	
	// Draw the rendered equation image at the bottom center
	if g.equationImage == nil {
		log.Fatal("Equation image is nil when trying to draw")
	}
	
	imgWidth := g.equationImage.Bounds().Dx()
	imgHeight := g.equationImage.Bounds().Dy()
	eqX := (screenWidth - imgWidth) / 2
	// place above instructions with small padding
	eqY := screenHeight - imgHeight - 25
	
	op := &ebiten.DrawImageOptions{}
	op.GeoM.Translate(float64(eqX), float64(eqY))
	screen.DrawImage(g.equationImage, op)
}

// buildLaTeXEquation builds the LaTeX equation string
func (g *GrammarForFiniteSentencesChapter) getValidRows() []int {
	validRows := []int{}
	for _, val := range g.rowValues {
		if val >= 0 {
			validRows = append(validRows, val)
		}
	}
	return validRows
}

// hashEquation creates a hash of the current equation for caching
func (g *GrammarForFiniteSentencesChapter) hashEquation() string {
	var hash strings.Builder
	for _, val := range g.rowValues {
		hash.WriteString(strconv.Itoa(val))
		hash.WriteString(",")
	}
	hash.WriteString(strconv.FormatFloat(g.globalCoeficient, 'f', -1, 64))
	hashBytes := sha256.Sum256([]byte(hash.String()))
	return hex.EncodeToString(hashBytes[:])
}

// renderEquationImage renders the equation as an image with proper mathematical notation
// Renders nested fractions: ((((2^v1-1)/c)*2^v2-1)/c)*2^v3-1)/c
// The structure is: each term wraps the previous result: ((previous) * 2^vi - 1) / c
// where c is the globalCoeficient
func (g *GrammarForFiniteSentencesChapter) renderEquationImage(validRows []int) (*ebiten.Image, error) {
	if len(validRows) == 0 {
		return nil, nil
	}
	
	textColor := color.RGBA{200, 200, 255, 255}
	lineColor := color.RGBA{200, 200, 255, 255}
	
	padding := 20.0
	lineHeight := 22.0
	horizontalSpacing := 8.0
	
	// Build from innermost to outermost iteratively
	// Track bounds of each nested level
	type FractionBounds struct {
		left, right, top, bottom float64
	}
	
	// Create temporary image for measurement
	testImg := ebiten.NewImage(1200, 300)
	
	currentX := padding
	currentY := padding + lineHeight
	var innerBounds *FractionBounds
	
	// Render innermost term first: (2^v1-1)/3
	if len(validRows) > 0 {
		exp := validRows[0]
		expStr := strconv.Itoa(exp)
		
		// Draw "2^exp - 1"
		twoText := "2"
		twoBounds := text.BoundString(basicfont.Face7x13, twoText)
		twoX := currentX
		twoY := currentY
		text.Draw(testImg, twoText, basicfont.Face7x13, int(twoX), int(twoY)+twoBounds.Dy(), textColor)
		
		// Draw superscript
		expBounds := text.BoundString(basicfont.Face7x13, expStr)
		expX := twoX + float64(twoBounds.Dx())
		expY := currentY - float64(expBounds.Dy())*0.6
		text.Draw(testImg, expStr, basicfont.Face7x13, int(expX), int(expY)+expBounds.Dy(), textColor)
		
		// Draw "-1"
		minusOneText := "-1"
		minusOneBounds := text.BoundString(basicfont.Face7x13, minusOneText)
		minusOneX := expX + float64(expBounds.Dx()) + 2
		text.Draw(testImg, minusOneText, basicfont.Face7x13, int(minusOneX), int(twoY)+minusOneBounds.Dy(), textColor)
		
		numEndX := minusOneX + float64(minusOneBounds.Dx())
		numTop := math.Min(twoY-float64(twoBounds.Dy()), expY-float64(expBounds.Dy()))
		numBottom := twoY + float64(twoBounds.Dy())
		
		// Draw denominator "3"
		denomText := strconv.FormatFloat(g.globalCoeficient, 'f', -1, 64)
		denomBounds := text.BoundString(basicfont.Face7x13, denomText)
		denomX := (currentX + numEndX - float64(denomBounds.Dx())) / 2
		denomY := numBottom + lineHeight
		text.Draw(testImg, denomText, basicfont.Face7x13, int(denomX), int(denomY)+denomBounds.Dy(), textColor)
		
		// Draw fraction line
		lineY := numBottom + lineHeight*0.7
		lineLeft := currentX - 4
		lineRight := numEndX + 4
		ebitenutil.DrawLine(testImg, lineLeft, lineY, lineRight, lineY, lineColor)
		
		innerBounds = &FractionBounds{
			left:   lineLeft,
			right:  numEndX,
			top:    numTop,
			bottom: denomY + float64(denomBounds.Dy()),
		}
	}
	
	// Now wrap each subsequent term around the previous one
	for termIndex := 1; termIndex < len(validRows); termIndex++ {
		exp := validRows[termIndex]
		expStr := strconv.Itoa(exp)
		
		// Draw opening parenthesis before the previous fraction
		parenText := "("
		parenBounds := text.BoundString(basicfont.Face7x13, parenText)
		parenX := innerBounds.left - float64(parenBounds.Dx()) - 2
		parenY := (innerBounds.top + innerBounds.bottom) / 2
		text.Draw(testImg, parenText, basicfont.Face7x13, int(parenX), int(parenY)+parenBounds.Dy(), textColor)
		
		// Draw multiplication dot
		dotX := innerBounds.right + horizontalSpacing
		dotY := (innerBounds.top + innerBounds.bottom) / 2
		ebitenutil.DrawRect(testImg, dotX-2, dotY-2, 4, 4, textColor)
		
		// Draw "2^exp"
		twoText := "2"
		twoBounds := text.BoundString(basicfont.Face7x13, twoText)
		twoX := dotX + horizontalSpacing
		twoY := dotY
		text.Draw(testImg, twoText, basicfont.Face7x13, int(twoX), int(twoY)+twoBounds.Dy(), textColor)
		
		// Draw superscript
		expBounds := text.BoundString(basicfont.Face7x13, expStr)
		expX := twoX + float64(twoBounds.Dx())
		expY := twoY - float64(expBounds.Dy())*0.6
		text.Draw(testImg, expStr, basicfont.Face7x13, int(expX), int(expY)+expBounds.Dy(), textColor)
		
		// Draw "-1"
		minusOneText := "-1"
		minusOneBounds := text.BoundString(basicfont.Face7x13, minusOneText)
		minusOneX := expX + float64(expBounds.Dx()) + 2
		text.Draw(testImg, minusOneText, basicfont.Face7x13, int(minusOneX), int(twoY)+minusOneBounds.Dy(), textColor)
		
		// Draw closing parenthesis
		parenCloseText := ")"
		parenCloseBounds := text.BoundString(basicfont.Face7x13, parenCloseText)
		parenCloseX := minusOneX + float64(minusOneBounds.Dx()) + 2
		text.Draw(testImg, parenCloseText, basicfont.Face7x13, int(parenCloseX), int(parenY)+parenCloseBounds.Dy(), textColor)
		
		numEndX := parenCloseX + float64(parenCloseBounds.Dx())
		numTop := math.Min(innerBounds.top, expY-float64(expBounds.Dy()))
		numBottom := math.Max(innerBounds.bottom, twoY+float64(twoBounds.Dy()))
		
		// Draw denominator "3"
		denomText := strconv.FormatFloat(g.globalCoeficient, 'f', -1, 64)
		denomBounds := text.BoundString(basicfont.Face7x13, denomText)
		denomX := (parenX + numEndX - float64(denomBounds.Dx())) / 2
		denomY := numBottom + lineHeight
		text.Draw(testImg, denomText, basicfont.Face7x13, int(denomX), int(denomY)+denomBounds.Dy(), textColor)
		
		// Draw fraction line spanning from parenX to numEndX
		lineY := numBottom + lineHeight*0.7
		ebitenutil.DrawLine(testImg, parenX-4, lineY, numEndX+4, lineY, lineColor)
		
		// Update bounds for next iteration
		innerBounds = &FractionBounds{
			left:   parenX,
			right:  numEndX,
			top:    numTop,
			bottom: denomY + float64(denomBounds.Dy()),
		}
	}
	
	// Create final image with proper size
	finalWidth := int(math.Ceil(innerBounds.right - innerBounds.left + padding*2))
	finalHeight := int(math.Ceil(innerBounds.bottom - innerBounds.top + padding*2))
	finalImg := ebiten.NewImage(finalWidth, finalHeight)
	
	// Re-render on final image
	currentX = padding
	currentY = padding + lineHeight - innerBounds.top
	innerBounds = nil
	
	// Render innermost term
	if len(validRows) > 0 {
		exp := validRows[0]
		expStr := strconv.Itoa(exp)
		
		twoText := "2"
		twoBounds := text.BoundString(basicfont.Face7x13, twoText)
		twoX := currentX
		twoY := currentY
		text.Draw(finalImg, twoText, basicfont.Face7x13, int(twoX), int(twoY)+twoBounds.Dy(), textColor)
		
		expBounds := text.BoundString(basicfont.Face7x13, expStr)
		expX := twoX + float64(twoBounds.Dx())
		expY := currentY - float64(expBounds.Dy())*0.6
		text.Draw(finalImg, expStr, basicfont.Face7x13, int(expX), int(expY)+expBounds.Dy(), textColor)
		
		minusOneText := "-1"
		minusOneBounds := text.BoundString(basicfont.Face7x13, minusOneText)
		minusOneX := expX + float64(expBounds.Dx()) + 2
		text.Draw(finalImg, minusOneText, basicfont.Face7x13, int(minusOneX), int(twoY)+minusOneBounds.Dy(), textColor)
		
		numEndX := minusOneX + float64(minusOneBounds.Dx())
		numTop := math.Min(twoY-float64(twoBounds.Dy()), expY-float64(expBounds.Dy()))
		numBottom := twoY + float64(twoBounds.Dy())
		
		denomText := strconv.FormatFloat(g.globalCoeficient, 'f', -1, 64)
		denomBounds := text.BoundString(basicfont.Face7x13, denomText)
		denomX := (currentX + numEndX - float64(denomBounds.Dx())) / 2
		denomY := numBottom + lineHeight
		text.Draw(finalImg, denomText, basicfont.Face7x13, int(denomX), int(denomY)+denomBounds.Dy(), textColor)
		
		lineY := numBottom + lineHeight*0.7
		lineLeft := currentX - 4
		lineRight := numEndX + 4
		ebitenutil.DrawLine(finalImg, lineLeft, lineY, lineRight, lineY, lineColor)
		
		innerBounds = &FractionBounds{
			left:   lineLeft,
			right:  numEndX,
			top:    numTop,
			bottom: denomY + float64(denomBounds.Dy()),
		}
	}
	
	// Wrap subsequent terms
	for termIndex := 1; termIndex < len(validRows); termIndex++ {
		exp := validRows[termIndex]
		expStr := strconv.Itoa(exp)
		
		parenText := "("
		parenBounds := text.BoundString(basicfont.Face7x13, parenText)
		parenX := innerBounds.left - float64(parenBounds.Dx()) - 2
		parenY := (innerBounds.top + innerBounds.bottom) / 2
		text.Draw(finalImg, parenText, basicfont.Face7x13, int(parenX), int(parenY)+parenBounds.Dy(), textColor)
		
		dotX := innerBounds.right + horizontalSpacing
		dotY := (innerBounds.top + innerBounds.bottom) / 2
		ebitenutil.DrawRect(finalImg, dotX-2, dotY-2, 4, 4, textColor)
		
		twoText := "2"
		twoBounds := text.BoundString(basicfont.Face7x13, twoText)
		twoX := dotX + horizontalSpacing
		twoY := dotY
		text.Draw(finalImg, twoText, basicfont.Face7x13, int(twoX), int(twoY)+twoBounds.Dy(), textColor)
		
		expBounds := text.BoundString(basicfont.Face7x13, expStr)
		expX := twoX + float64(twoBounds.Dx())
		expY := twoY - float64(expBounds.Dy())*0.6
		text.Draw(finalImg, expStr, basicfont.Face7x13, int(expX), int(expY)+expBounds.Dy(), textColor)
		
		minusOneText := "-1"
		minusOneBounds := text.BoundString(basicfont.Face7x13, minusOneText)
		minusOneX := expX + float64(expBounds.Dx()) + 2
		text.Draw(finalImg, minusOneText, basicfont.Face7x13, int(minusOneX), int(twoY)+minusOneBounds.Dy(), textColor)
		
		parenCloseText := ")"
		parenCloseBounds := text.BoundString(basicfont.Face7x13, parenCloseText)
		parenCloseX := minusOneX + float64(minusOneBounds.Dx()) + 2
		text.Draw(finalImg, parenCloseText, basicfont.Face7x13, int(parenCloseX), int(parenY)+parenCloseBounds.Dy(), textColor)
		
		numEndX := parenCloseX + float64(parenCloseBounds.Dx())
		numTop := math.Min(innerBounds.top, expY-float64(expBounds.Dy()))
		numBottom := math.Max(innerBounds.bottom, twoY+float64(twoBounds.Dy()))
		
		denomText := strconv.FormatFloat(g.globalCoeficient, 'f', -1, 64)
		denomBounds := text.BoundString(basicfont.Face7x13, denomText)
		denomX := (parenX + numEndX - float64(denomBounds.Dx())) / 2
		denomY := numBottom + lineHeight
		text.Draw(finalImg, denomText, basicfont.Face7x13, int(denomX), int(denomY)+denomBounds.Dy(), textColor)
		
		lineY := numBottom + lineHeight*0.7
		ebitenutil.DrawLine(finalImg, parenX-4, lineY, numEndX+4, lineY, lineColor)
		
		innerBounds = &FractionBounds{
			left:   parenX,
			right:  numEndX,
			top:    numTop,
			bottom: denomY + float64(denomBounds.Dy()),
		}
	}
	
	return finalImg, nil
}

// cycleGlobalCoeficient cycles through odd numbers from 1 to 21
func (g *GrammarForFiniteSentencesChapter) cycleGlobalCoeficient() {
	// Odd numbers from 1 to 21
	oddNumbers := []float64{1, 3, 5, 7, 9, 11, 13, 15, 17, 19, 21}
	
	// Find current index
	currentIndex := -1
	for i, val := range oddNumbers {
		if val == g.globalCoeficient {
			currentIndex = i
			break
		}
	}
	
	// If not found, default to index 1 (value 3)
	if currentIndex == -1 {
		currentIndex = 1
	}
	
	// Cycle to next value
	currentIndex = (currentIndex + 1) % len(oddNumbers)
	g.globalCoeficient = oddNumbers[currentIndex]
}

// calculatePartialIndexReverse calculates the index using the first n rows
// n is the number of rows to use (from row 1 to row n, i.e., array indices 0 to n-1)
// Note: Display shows rows in reverse order, but calculation always processes from row 1 (index 0) first
func (g *GrammarForFiniteSentencesChapter) calculatePartialIndexReverse(n int) *big.Int {
	if n == 0 {
		return big.NewInt(1)
	}
	
	if len(g.rowValues) == 0 {
		return big.NewInt(1)
	}
	
	// Use the first n rows (row 1 to row n, array indices 0 to n-1)
	maxRows := n
	if maxRows > len(g.rowValues) {
		maxRows = len(g.rowValues)
	}
	
	// Start with first value (row 1, array index 0)
	firstValue := g.rowValues[0]
	if firstValue < 0 || maxRows == 0 {
		return big.NewInt(1)
	}
	
	// Calculate (2^firstValue - 1) / globalCoeficient
	coef := big.NewInt(int64(g.globalCoeficient))
	one := big.NewInt(1)
	
	// 2^firstValue
	powerOf2 := new(big.Int).Lsh(one, uint(firstValue))
	// 2^firstValue - 1
	powerOf2Minus1 := new(big.Int).Sub(powerOf2, one)
	// (2^firstValue - 1) / globalCoeficient
	result := new(big.Int).Div(powerOf2Minus1, coef)
	
	// Check if division is exact
	remainder := new(big.Int).Mod(powerOf2Minus1, coef)
	if remainder.Sign() != 0 {
		return big.NewInt(0)
	}
	
	// Continue with remaining values up to n (from row 2 onwards, array index 1 onwards)
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
		
		// Divide by globalCoeficient
		remainder := new(big.Int).Mod(result, coef)
		if remainder.Sign() != 0 {
			return big.NewInt(0)
		}
		result.Div(result, coef)
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
