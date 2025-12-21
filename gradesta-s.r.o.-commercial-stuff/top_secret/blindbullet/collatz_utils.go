package main

import (
	"image/color"
	"math"
	"strconv"

	"github.com/hajimehoshi/ebiten/v2"
	"github.com/hajimehoshi/ebiten/v2/ebitenutil"
	"github.com/hajimehoshi/ebiten/v2/text"
	"golang.org/x/image/font/basicfont"
)

// findLargestPowerOf2 finds the largest k such that y is divisible by 2^k
// k is the number of times y is divisible by 2
// Works correctly for both positive and negative numbers
func findLargestPowerOf2(y int) int {
	if y == 0 {
		return 0
	}
	// Work with absolute value to handle negative numbers correctly
	absY := y
	if absY < 0 {
		absY = -absY
	}
	k := 0
	for absY%2 == 0 {
		absY /= 2
		k++
	}
	return k
}

// StairStep represents one step in the staircase
type StairStep struct {
	StartIndex      int     // Starting index for this step
	DestinationIndex float64 // Destination index (may be fractional)
	K               int     // Power of 2 for this step
}

// CollatzTableData contains the data needed to draw the Collatz table
type CollatzTableData struct {
	Steps      []StairStep // The staircase steps
	StartIndex int         // Starting index
	Coefficient float64    // Global coefficient (for y = coefficient*x + 1)
	IsUpwards  bool        // Whether staircase goes upwards
	HitLimit   bool        // Whether step limit was reached
	StopOnDirectionChange bool // If true, stop when direction changes; if false, continue up to maxSteps
}

// DrawCollatzTable draws the Collatz table showing steps, index, y, k, and destination values
func DrawCollatzTable(screen *ebiten.Image, data CollatzTableData) {
	stepsCount := len(data.Steps)
	
	// Draw table header at top left (with padding from top to avoid overlap with summary text)
	headerY := 55
	lineHeight := 13
	
	// Column positions (left-aligned)
	colStepX := 10
	colIndexX := 60
	colYValueX := 130
	colKX := 200
	colDestX := 240
	
	// Draw header row
	text.Draw(screen, "Step", basicfont.Face7x13, colStepX, headerY, color.RGBA{200, 200, 255, 255})
	text.Draw(screen, "Index", basicfont.Face7x13, colIndexX, headerY, color.RGBA{200, 200, 255, 255})
	text.Draw(screen, "Y", basicfont.Face7x13, colYValueX, headerY, color.RGBA{200, 200, 255, 255})
	text.Draw(screen, "K", basicfont.Face7x13, colKX, headerY, color.RGBA{200, 200, 255, 255})
	text.Draw(screen, "Dest", basicfont.Face7x13, colDestX, headerY, color.RGBA{200, 200, 255, 255})
	
	// Draw separator line
	headerY += lineHeight + 2
	for x := 10; x < screenWidth-10; x++ {
		ebitenutil.DrawRect(screen, float64(x), float64(headerY), 1, 1, color.Gray{Y: 100})
	}
	headerY += 2
	
	// Draw table rows - include starting point as row 0, then all steps
	currentIndex := float64(data.StartIndex)
	maxRows := 50 // Limit number of visible rows to fit on screen
	totalRows := stepsCount + 1 // Include starting point
	startRow := 0
	if totalRows > maxRows {
		// If too many rows, show the last maxRows
		startRow = totalRows - maxRows
	}
	
	rowNum := 0
	// Draw starting point (row 0) only if there are no steps
	// If there are steps, the starting point will be shown as step 1's starting index
	if stepsCount == 0 && startRow == 0 {
		rowY := headerY + lineHeight*rowNum
		if rowY < screenHeight-50 {
			startYValue := data.Coefficient*currentIndex + 1.0
			startYValueInt := int(startYValue)
			startK := findLargestPowerOf2(startYValueInt)
			
			// Draw step number
			text.Draw(screen, "0", basicfont.Face7x13, colStepX, rowY, color.RGBA{255, 255, 100, 255}) // Yellow for start
			
			// Draw index
			indexText := strconv.Itoa(int(currentIndex))
			text.Draw(screen, indexText, basicfont.Face7x13, colIndexX, rowY, color.RGBA{255, 255, 100, 255})
			
			// Draw Y value
			yText := strconv.FormatFloat(startYValue, 'f', 1, 64)
			text.Draw(screen, yText, basicfont.Face7x13, colYValueX, rowY, color.RGBA{255, 255, 100, 255})
			
			// Draw K value
			kText := strconv.Itoa(startK)
			text.Draw(screen, kText, basicfont.Face7x13, colKX, rowY, color.RGBA{255, 255, 100, 255})
			
			// Draw destination (if K > 0)
			if startK > 0 {
				powerOf2 := math.Pow(2.0, float64(startK))
				destinationIndex := float64(startYValueInt) / powerOf2
				destText := strconv.FormatFloat(destinationIndex, 'f', 1, 64)
				text.Draw(screen, destText, basicfont.Face7x13, colDestX, rowY, color.RGBA{100, 255, 255, 255}) // Cyan
			} else {
				text.Draw(screen, "-", basicfont.Face7x13, colDestX, rowY, color.Gray{Y: 100})
			}
		}
		rowNum++
	}
	
	// Update currentIndex for skipped steps
	for i := 0; i < startRow-1 && i < stepsCount; i++ {
		currentIndex = data.Steps[i].DestinationIndex
	}
	
	// Draw step rows
	for i := startRow - 1; i < stepsCount; i++ {
		if i < 0 {
			continue // Skip if we're before the first step
		}
		
		step := data.Steps[i]
		rowY := headerY + lineHeight*rowNum
		
		if rowY >= screenHeight-50 {
			break // Stop if we've run out of screen space
		}
		
		// Calculate values for this step
		stepYValue := data.Coefficient*currentIndex + 1.0
		stepK := step.K
		stepDest := step.DestinationIndex
		
		// Draw step number
		stepText := strconv.Itoa(i + 1)
		text.Draw(screen, stepText, basicfont.Face7x13, colStepX, rowY, color.White)
		
		// Draw index
		indexText := strconv.Itoa(int(currentIndex))
		text.Draw(screen, indexText, basicfont.Face7x13, colIndexX, rowY, color.White)
		
		// Draw Y value
		yText := strconv.FormatFloat(stepYValue, 'f', 1, 64)
		text.Draw(screen, yText, basicfont.Face7x13, colYValueX, rowY, color.White)
		
		// Draw K value
		kText := strconv.Itoa(stepK)
		text.Draw(screen, kText, basicfont.Face7x13, colKX, rowY, color.White)
		
		// Draw destination
		destText := strconv.FormatFloat(stepDest, 'f', 1, 64)
		text.Draw(screen, destText, basicfont.Face7x13, colDestX, rowY, color.RGBA{100, 255, 255, 255}) // Cyan
		
		// Move to destination for next iteration
		currentIndex = step.DestinationIndex
		rowNum++
	}
	
	// Draw summary info above the table (with padding)
	summaryY := 20
	// Draw global coefficient
	coefText := "Coefficient: " + strconv.FormatFloat(data.Coefficient, 'f', 0, 64) + " (Press C to change)"
	text.Draw(screen, coefText, basicfont.Face7x13, 10, summaryY, color.RGBA{100, 255, 100, 255}) // Green
	
	if stepsCount > 0 {
		endIndex := int(data.Steps[stepsCount-1].DestinationIndex)
		var directionText string
		if data.IsUpwards {
			directionText = "top: " + strconv.Itoa(endIndex)
		} else {
			directionText = "bottom: " + strconv.Itoa(endIndex)
		}
		stepsText := "Stairs: " + strconv.Itoa(stepsCount) + " steps (" + directionText + ")"
		if data.HitLimit {
			stepsText += " [LIMIT REACHED]"
		}
		if stepsCount > maxRows {
			stepsText += " (showing last " + strconv.Itoa(maxRows) + ")"
		}
		text.Draw(screen, stepsText, basicfont.Face7x13, 10, summaryY+13, color.RGBA{255, 200, 100, 255}) // Orange
	} else {
		// No staircase - show current point info
		yValue := data.Coefficient*float64(data.StartIndex) + 1.0
		yValueInt := int(yValue)
		k := findLargestPowerOf2(yValueInt)
		
		infoText := "Index: " + strconv.Itoa(data.StartIndex) + " | Y: " + strconv.FormatFloat(yValue, 'f', 1, 64) + " | K: " + strconv.Itoa(k)
		if k > 0 {
			powerOf2 := math.Pow(2.0, float64(k))
			destinationIndex := float64(yValueInt) / powerOf2
			infoText += " | Dest: " + strconv.FormatFloat(destinationIndex, 'f', 1, 64)
		}
		text.Draw(screen, infoText, basicfont.Face7x13, 10, summaryY+13, color.White)
	}
}

// CalculateStaircase calculates all steps from startIndex
// If stopOnDirectionChange is true, it stops when the direction changes
// If false, it continues up to maxSteps even if direction changes
func CalculateStaircase(startIndex int, coefficient float64, maxSteps int, stopOnDirectionChange bool) ([]StairStep, bool, bool) {
	var steps []StairStep
	currentIndex := float64(startIndex)
	isUpwards := false
	hitLimit := false
	
	for len(steps) < maxSteps {
		// Calculate y value for current index
		yValue := coefficient*currentIndex + 1.0
		yValueInt := int(yValue)
		
		// Find k (number of times y is divisible by 2)
		k := findLargestPowerOf2(yValueInt)
		if k == 0 {
			break // No destination if y is odd
		}
		
		// Calculate destination index
		powerOf2 := math.Pow(2.0, float64(k))
		destinationIndex := float64(yValueInt) / powerOf2
		
		// Check if destination is different from current index
		if destinationIndex == currentIndex {
			break // Reached the end of the stairs
		}
		
		// Determine direction on first step
		if len(steps) == 0 {
			isUpwards = destinationIndex > currentIndex
		}
		
		// Check if we're still going in the same direction (only if stopOnDirectionChange is true)
		if stopOnDirectionChange {
			if isUpwards && destinationIndex <= currentIndex {
				break // Reached the top of the stairs
			}
			if !isUpwards && destinationIndex >= currentIndex {
				break // Reached the bottom of the stairs
			}
		}
		
		// Add this step
		steps = append(steps, StairStep{
			StartIndex:      int(currentIndex),
			DestinationIndex: destinationIndex,
			K:               k,
		})
		
		// Move to destination for next iteration
		currentIndex = destinationIndex
	}
	
	// Check if we hit the limit
	if len(steps) >= maxSteps {
		hitLimit = true
	}
	
	return steps, isUpwards, hitLimit
}

