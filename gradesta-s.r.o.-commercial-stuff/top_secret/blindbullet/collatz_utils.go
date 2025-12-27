package main

import (
	"image/color"
	"math/big"
	"strconv"

	"github.com/hajimehoshi/ebiten/v2"
	"github.com/hajimehoshi/ebiten/v2/text"
	"golang.org/x/image/font/basicfont"
)

// formatNumber formats a number string to fit in 10 digits, using scientific notation if needed
func formatNumber(numStr string) string {
	if len(numStr) <= 10 {
		return numStr
	}
	// Convert to scientific notation
	// Try to parse as big.Int to get the number
	num, ok := new(big.Int).SetString(numStr, 10)
	if !ok {
		return numStr // If parsing fails, return original
	}
	
	// Convert to float64 for scientific notation
	numFloat := new(big.Float).SetInt(num)
	f64, _ := numFloat.Float64()
	
	// Format with scientific notation, ensuring it fits in 10 characters
	// Format: -1.23e+45 (max 10 chars: -1.23e+45 = 9, -1.23e+123 = 10)
	sciNotation := strconv.FormatFloat(f64, 'e', 2, 64)
	
	// If scientific notation is still too long, try fewer decimal places
	if len(sciNotation) > 10 {
		sciNotation = strconv.FormatFloat(f64, 'e', 1, 64)
	}
	if len(sciNotation) > 10 {
		sciNotation = strconv.FormatFloat(f64, 'e', 0, 64)
	}
	
	return sciNotation
}

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

// findLargestPowerOf2Big finds the largest k such that y is divisible by 2^k using big.Int
func findLargestPowerOf2Big(y *big.Int) int {
	if y.Sign() == 0 {
		return 0
	}
	// Work with absolute value
	absY := new(big.Int).Abs(y)
	k := 0
	two := big.NewInt(2)
	zero := big.NewInt(0)
	for {
		mod := new(big.Int).Mod(absY, two)
		if mod.Cmp(zero) != 0 {
			break
		}
		absY.Div(absY, two)
		k++
	}
	return k
}

// StairStep represents one step in the staircase
type StairStep struct {
	StartIndex      *big.Int // Starting index for this step
	DestinationIndex *big.Int // Destination index
	K               int     // Power of 2 for this step
}

// CollatzTableData contains the data needed to draw the Collatz table
type CollatzTableData struct {
	Steps      []StairStep // The staircase steps
	StartIndex *big.Int    // Starting index
	Coefficient float64    // Global coefficient (for y = coefficient*x + 1)
	IsUpwards  bool        // Whether staircase goes upwards
	HitLimit   bool        // Whether step limit was reached
	StopOnDirectionChange bool // If true, stop when direction changes; if false, continue up to maxSteps
}

// DrawCollatzTable draws the Collatz table showing steps, index, y, k, and destination values
func DrawCollatzTable(screen *ebiten.Image, data CollatzTableData) {
	DrawCollatzTableAt(screen, data, 10, 55)
}

// DrawCollatzTableAt draws the Collatz table at a specific X, Y position
func DrawCollatzTableAt(screen *ebiten.Image, data CollatzTableData, offsetX, offsetY int) {
	stepsCount := len(data.Steps)
	
	// Draw table header at specified position
	headerY := offsetY
	lineHeight := 13
	
	// Column positions (left-aligned, offset by offsetX)
	colStepX := offsetX
	colIndexX := offsetX + 50
	colYValueX := offsetX + 120
	colKX := offsetX + 190
	colDestX := offsetX + 230
	
	// Draw header row
	text.Draw(screen, "Step", basicfont.Face7x13, colStepX, headerY, color.RGBA{200, 200, 255, 255})
	text.Draw(screen, "Index", basicfont.Face7x13, colIndexX, headerY, color.RGBA{200, 200, 255, 255})
	text.Draw(screen, "Y", basicfont.Face7x13, colYValueX, headerY, color.RGBA{200, 200, 255, 255})
	text.Draw(screen, "K", basicfont.Face7x13, colKX, headerY, color.RGBA{200, 200, 255, 255})
	text.Draw(screen, "Dest", basicfont.Face7x13, colDestX, headerY, color.RGBA{200, 200, 255, 255})
	
	// Move headerY down for first row (no separator line)
	headerY += lineHeight + 2
	
	// Draw table rows - show all steps, or special case for indexes with no steps (like 1, -1)
	rowNum := 0
	currentIndexBig := new(big.Int).Set(data.StartIndex)
	
	// Special case: if there are no steps, show the starting point and its destination
	if stepsCount == 0 {
		rowY := headerY + lineHeight*rowNum
		if rowY < screenHeight-50 {
			// Calculate Y as big.Int
			coefBig := big.NewInt(int64(data.Coefficient))
			startYValueBig := new(big.Int).Mul(coefBig, currentIndexBig)
			startYValueBig.Add(startYValueBig, big.NewInt(1))
			startK := findLargestPowerOf2Big(startYValueBig)
			
			// Draw step number
			text.Draw(screen, "1", basicfont.Face7x13, colStepX, rowY, color.White)
			
			// Draw index
			indexText := formatNumber(currentIndexBig.String())
			text.Draw(screen, indexText, basicfont.Face7x13, colIndexX, rowY, color.White)
			
			// Draw Y value as integer (with scientific notation if needed)
			yText := formatNumber(startYValueBig.String())
			text.Draw(screen, yText, basicfont.Face7x13, colYValueX, rowY, color.White)
			
			// Draw K value
			kText := strconv.Itoa(startK)
			text.Draw(screen, kText, basicfont.Face7x13, colKX, rowY, color.White)
			
			// Draw destination (if K > 0) as big.Int (with scientific notation if needed)
			if startK > 0 {
				powerOf2Big := new(big.Int).Lsh(big.NewInt(1), uint(startK)) // 1 << k
				destinationIndexBig := new(big.Int).Div(startYValueBig, powerOf2Big)
				destText := formatNumber(destinationIndexBig.String())
				text.Draw(screen, destText, basicfont.Face7x13, colDestX, rowY, color.RGBA{100, 255, 255, 255}) // Cyan
			} else {
				text.Draw(screen, "-", basicfont.Face7x13, colDestX, rowY, color.Gray{Y: 100})
			}
		}
		rowNum++
	} else {
		// Draw all step rows
		for i := 0; i < stepsCount; i++ {
			step := data.Steps[i]
			rowY := headerY + lineHeight*rowNum
			
			if rowY >= screenHeight-50 {
				break // Stop if we've run out of screen space
			}
			
			// Use the stored StartIndex from the step (this is the actual starting index)
			// Calculate Y value from StartIndex using big.Int
			coefBig := big.NewInt(int64(data.Coefficient))
			stepYValueBig := new(big.Int).Mul(coefBig, step.StartIndex)
			stepYValueBig.Add(stepYValueBig, big.NewInt(1))
			
			stepK := step.K
			
			// Draw step number
			stepText := strconv.Itoa(i + 1)
			text.Draw(screen, stepText, basicfont.Face7x13, colStepX, rowY, color.White)
			
			// Draw index - use step.StartIndex (the actual starting index for this step)
			indexText := formatNumber(step.StartIndex.String())
			text.Draw(screen, indexText, basicfont.Face7x13, colIndexX, rowY, color.White)
			
			// Draw Y value as integer (with scientific notation if needed)
			yText := formatNumber(stepYValueBig.String())
			text.Draw(screen, yText, basicfont.Face7x13, colYValueX, rowY, color.White)
			
			// Draw K value
			kText := strconv.Itoa(stepK)
			text.Draw(screen, kText, basicfont.Face7x13, colKX, rowY, color.White)
			
			// Draw destination as integer (with scientific notation if needed)
			destText := formatNumber(step.DestinationIndex.String())
			text.Draw(screen, destText, basicfont.Face7x13, colDestX, rowY, color.RGBA{100, 255, 255, 255}) // Cyan
			
			rowNum++
		}
	}
	
	// Draw summary info above the table (with padding) - only if at default position
	if offsetX == 10 && offsetY == 55 {
		summaryY := 20
		// Draw global coefficient
		coefText := "Coefficient: " + strconv.FormatFloat(data.Coefficient, 'f', 0, 64) + " (Press C to change)"
		text.Draw(screen, coefText, basicfont.Face7x13, 10, summaryY, color.RGBA{100, 255, 100, 255}) // Green
		
		if stepsCount > 0 {
			endIndexText := formatNumber(data.Steps[stepsCount-1].DestinationIndex.String())
			var directionText string
			if data.IsUpwards {
				directionText = "top: " + endIndexText
			} else {
				directionText = "bottom: " + endIndexText
			}
		stepsText := "Stairs: " + strconv.Itoa(stepsCount) + " steps (" + directionText + ")"
		if data.HitLimit {
			stepsText += " [LIMIT REACHED]"
		}
			text.Draw(screen, stepsText, basicfont.Face7x13, 10, summaryY+13, color.RGBA{255, 200, 100, 255}) // Orange
		} else {
			// No staircase - show current point info using big.Int
			coefBig := big.NewInt(int64(data.Coefficient))
			startIndexBig := new(big.Int).Set(data.StartIndex)
			yValueBig := new(big.Int).Mul(coefBig, startIndexBig)
			yValueBig.Add(yValueBig, big.NewInt(1))
			k := findLargestPowerOf2Big(yValueBig)
			
			yText := formatNumber(yValueBig.String())
			indexText := formatNumber(startIndexBig.String())
			infoText := "Index: " + indexText + " | Y: " + yText + " | K: " + strconv.Itoa(k)
			if k > 0 {
				powerOf2Big := new(big.Int).Lsh(big.NewInt(1), uint(k))
				destinationIndexBig := new(big.Int).Div(yValueBig, powerOf2Big)
				destText := formatNumber(destinationIndexBig.String())
				infoText += " | Dest: " + destText
			}
			text.Draw(screen, infoText, basicfont.Face7x13, 10, summaryY+13, color.White)
		}
	}
}

// CalculateStaircase calculates all steps from startIndex using big.Int for precision
// If stopOnDirectionChange is true, it stops when the direction changes
// If false, it continues up to maxSteps even if direction changes
func CalculateStaircase(startIndex *big.Int, coefficient float64, maxSteps int, stopOnDirectionChange bool) ([]StairStep, bool, bool) {
	var steps []StairStep
	currentIndexBig := new(big.Int).Set(startIndex)
	coefBig := big.NewInt(int64(coefficient))
	isUpwards := false
	hitLimit := false
	one := big.NewInt(1)
	
	for len(steps) < maxSteps {
		// Calculate y value for current index using big.Int
		yValueBig := new(big.Int).Mul(coefBig, currentIndexBig)
		yValueBig.Add(yValueBig, one)
		
		// Find k (number of times y is divisible by 2) using big.Int
		k := findLargestPowerOf2Big(yValueBig)
		if k == 0 {
			break // No destination if y is odd
		}
		
		// Calculate destination index using big.Int
		powerOf2Big := new(big.Int).Lsh(one, uint(k)) // 1 << k
		destinationIndexBig := new(big.Int).Div(yValueBig, powerOf2Big)
		
		// Check if destination is different from current index
		if destinationIndexBig.Cmp(currentIndexBig) == 0 {
			break // Reached the end of the stairs
		}
		
		// Determine direction on first step
		if len(steps) == 0 {
			isUpwards = destinationIndexBig.Cmp(currentIndexBig) > 0
		}
		
		// Check if we're still going in the same direction (only if stopOnDirectionChange is true)
		if stopOnDirectionChange {
			if isUpwards && destinationIndexBig.Cmp(currentIndexBig) <= 0 {
				break // Reached the top of the stairs
			}
			if !isUpwards && destinationIndexBig.Cmp(currentIndexBig) >= 0 {
				break // Reached the bottom of the stairs
			}
		}
		
		// Add this step (store as big.Int)
		steps = append(steps, StairStep{
			StartIndex:      new(big.Int).Set(currentIndexBig),
			DestinationIndex: new(big.Int).Set(destinationIndexBig),
			K:               k,
		})
		
		// Move to destination for next iteration
		currentIndexBig.Set(destinationIndexBig)
	}
	
	// Check if we hit the limit
	if len(steps) >= maxSteps {
		hitLimit = true
	}
	
	return steps, isUpwards, hitLimit
}

// CalculateStaircaseExtended calculates the staircase and extends it beyond the destination
// by continuing from the destination index for additionalSteps more steps
// This is useful when you need k values beyond what the normal staircase provides
func CalculateStaircaseExtended(startIndex *big.Int, coefficient float64, maxSteps int, stopOnDirectionChange bool, additionalSteps int) ([]StairStep, bool, bool) {
	// First, calculate the normal staircase
	steps, isUpwards, hitLimit := CalculateStaircase(startIndex, coefficient, maxSteps, stopOnDirectionChange)
	
	// If we need additional steps and we have a destination, continue from there
	if additionalSteps > 0 && len(steps) > 0 {
		// Get the destination index from the last step
		destinationIndex := steps[len(steps)-1].DestinationIndex
		
		// Continue calculating steps from the destination index
		// We'll continue until we have enough steps or hit a limit
		currentIndex := new(big.Int).Set(destinationIndex)
		coefBig := big.NewInt(int64(coefficient))
		one := big.NewInt(1)
		
		for i := 0; i < additionalSteps && len(steps) < maxSteps+additionalSteps; i++ {
			// Calculate y value for current index
			yValueBig := new(big.Int).Mul(coefBig, currentIndex)
			yValueBig.Add(yValueBig, one)
			
			// Find k
			k := findLargestPowerOf2Big(yValueBig)
			if k == 0 {
				break // No destination if y is odd
			}
			
			// Calculate destination index
			powerOf2Big := new(big.Int).Lsh(one, uint(k))
			destinationIndexBig := new(big.Int).Div(yValueBig, powerOf2Big)
			
			// Check if destination is different from current index
			if destinationIndexBig.Cmp(currentIndex) == 0 {
				break // Reached the end
			}
			
			// Add this step
			steps = append(steps, StairStep{
				StartIndex:      new(big.Int).Set(currentIndex),
				DestinationIndex: new(big.Int).Set(destinationIndexBig),
				K:               k,
			})
			
			// Move to destination for next iteration
			currentIndex.Set(destinationIndexBig)
		}
	}
	
	return steps, isUpwards, hitLimit
}

