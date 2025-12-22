package main

import (
	"fmt"
	"image/color"
	"math"
	"math/big"
	"strings"

	"github.com/hajimehoshi/ebiten/v2"
	"github.com/hajimehoshi/ebiten/v2/ebitenutil"
	"github.com/hajimehoshi/ebiten/v2/inpututil"
	"github.com/hajimehoshi/ebiten/v2/text"
	"golang.org/x/image/font/basicfont"
)

const (
	stepsPerTurn = 32  // Total steps per turn (16 odd steps per turn)
	spiralRadius = 200.0 // Radius of the spiral in pixels
	stepHeight   = 10.0  // Height difference between steps in world units
)

// SpiralStairsChapter implements the Spiral Staircase chapter
type SpiralStairsChapter struct {
	currentStep      *big.Int // Current step (0-based, can be negative)
	zoom             float64   // Zoom level
	globalCoeficient float64   // Coefficient for the line equation (y = globalCoeficient*x + 1)
	
	// Cached staircase calculation
	cachedSteps      []StairStep
	cachedIsUpwards  bool
	cachedHitLimit   bool
	cachedForIndex   *big.Int // Cache key for currentStep
	cachedForCoef    float64
	
	// Cached frequency jump for shift-hold behavior
	cachedFrequencyJump *big.Int // Jump amount (frequency * 2) set by 'f' key
	
	// Jump input mode
	jumpInputMode bool   // Whether we're in jump input mode
	jumpInputBuffer string // Buffer for typing step number
	
	// Help dialog
	showHelp bool // Whether help dialog is visible
	helpScrollOffset int // Scroll offset for help dialog
	escConsumed bool // Whether ESC was consumed by a dialog this frame
}

// NewSpiralStairsChapter creates a new Spiral Staircase chapter
func NewSpiralStairsChapter() *SpiralStairsChapter {
	return &SpiralStairsChapter{
		currentStep:      big.NewInt(1), // Start at an odd step
		zoom:             1.0,
		globalCoeficient: 3.0,           // y = 3x + 1
		cachedForIndex:   nil,           // Cache invalid initially
	}
}

func (s *SpiralStairsChapter) Update() error {
	s.escConsumed = false // Reset at start of frame
	
	// Handle 'h' key to toggle help dialog
	if inpututil.IsKeyJustPressed(ebiten.KeyH) {
		s.showHelp = !s.showHelp
		if s.showHelp {
			s.helpScrollOffset = 0
		}
		return nil
	}
	
	// Handle help dialog scrolling
	if s.showHelp {
		if inpututil.IsKeyJustPressed(ebiten.KeyArrowDown) || inpututil.IsKeyJustPressed(ebiten.KeyS) {
			s.helpScrollOffset += 15
		}
		if inpututil.IsKeyJustPressed(ebiten.KeyArrowUp) || inpututil.IsKeyJustPressed(ebiten.KeyW) {
			s.helpScrollOffset -= 15
			if s.helpScrollOffset < 0 {
				s.helpScrollOffset = 0
			}
		}
		// Close help with Escape or 'h' again
		if inpututil.IsKeyJustPressed(ebiten.KeyEscape) || inpututil.IsKeyJustPressed(ebiten.KeyH) {
			s.showHelp = false
			s.escConsumed = true
		}
		// Don't process other keys when help is open
		return nil
	}
	
	// Handle 'c' key to cycle through global coefficient values (odd numbers 1-21)
	if inpututil.IsKeyJustPressed(ebiten.KeyC) {
		s.cycleGlobalCoeficient()
	}
	
	// Handle 'f' key to update frequency jump based on current step
	if inpututil.IsKeyJustPressed(ebiten.KeyF) {
		steps, _, _ := s.getCachedStaircase()
		frequency := s.calculateFrequency(steps)
		// Multiply by 2 to stay on odd steps
		s.cachedFrequencyJump = new(big.Int).Mul(frequency, big.NewInt(2))
		return nil
	}
	
	// Handle 'j' key to enter jump input mode
	if inpututil.IsKeyJustPressed(ebiten.KeyJ) {
		s.jumpInputMode = true
		s.jumpInputBuffer = ""
		return nil
	}
	
	// Handle jump input mode
	if s.jumpInputMode {
		// Handle Escape to cancel
		if inpututil.IsKeyJustPressed(ebiten.KeyEscape) {
			s.jumpInputMode = false
			s.jumpInputBuffer = ""
			s.escConsumed = true
			return nil
		}
		
		// Handle Enter to confirm jump
		if inpututil.IsKeyJustPressed(ebiten.KeyEnter) {
			if s.jumpInputBuffer != "" {
				// Parse the input as big.Int
				targetStep := new(big.Int)
				if _, ok := targetStep.SetString(s.jumpInputBuffer, 10); ok {
					s.currentStep.Set(targetStep)
					// Ensure we stay on odd step
					mod := new(big.Int).Mod(s.currentStep, big.NewInt(2))
					if mod.Sign() == 0 {
						s.currentStep.Add(s.currentStep, big.NewInt(1))
					}
				}
			}
			s.jumpInputMode = false
			s.jumpInputBuffer = ""
			return nil
		}
		
		// Handle Backspace to delete last character
		if inpututil.IsKeyJustPressed(ebiten.KeyBackspace) {
			if len(s.jumpInputBuffer) > 0 {
				s.jumpInputBuffer = s.jumpInputBuffer[:len(s.jumpInputBuffer)-1]
			}
			return nil
		}
		
		// Handle numeric keys (0-9)
		keys := []ebiten.Key{ebiten.Key0, ebiten.Key1, ebiten.Key2, ebiten.Key3, ebiten.Key4,
			ebiten.Key5, ebiten.Key6, ebiten.Key7, ebiten.Key8, ebiten.Key9}
		for i, key := range keys {
			if inpututil.IsKeyJustPressed(key) {
				s.jumpInputBuffer += string(rune('0' + i))
				return nil
			}
		}
		
		// Handle minus sign (only at the start)
		if inpututil.IsKeyJustPressed(ebiten.KeyMinus) && len(s.jumpInputBuffer) == 0 {
			s.jumpInputBuffer = "-"
			return nil
		}
		
		// In jump input mode, ignore other keys
		return nil
	}
	
	// Handle 'r' key to reset to step 1
	if inpututil.IsKeyJustPressed(ebiten.KeyR) {
		s.currentStep.SetInt64(1)
		return nil
	}
	
	// Handle shift+arrow keys to jump by cached frequency
	shiftPressedForJump := ebiten.IsKeyPressed(ebiten.KeyShiftLeft) || ebiten.IsKeyPressed(ebiten.KeyShiftRight)
	
	if shiftPressedForJump && s.cachedFrequencyJump != nil {
		if inpututil.IsKeyJustPressed(ebiten.KeyArrowRight) || inpututil.IsKeyJustPressed(ebiten.KeyD) {
			s.currentStep.Add(s.currentStep, s.cachedFrequencyJump)
			// Ensure we stay on odd step
			mod := new(big.Int).Mod(s.currentStep, big.NewInt(2))
			if mod.Sign() == 0 {
				s.currentStep.Add(s.currentStep, big.NewInt(1))
			}
			return nil
		}
		if inpututil.IsKeyJustPressed(ebiten.KeyArrowLeft) || inpututil.IsKeyJustPressed(ebiten.KeyA) {
			s.currentStep.Sub(s.currentStep, s.cachedFrequencyJump)
			// Ensure we stay on odd step
			mod := new(big.Int).Mod(s.currentStep, big.NewInt(2))
			if mod.Sign() == 0 {
				s.currentStep.Sub(s.currentStep, big.NewInt(1))
			}
			return nil
		}
	}
	
	// Handle left/right movement - only allow odd steps
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowRight) || inpututil.IsKeyJustPressed(ebiten.KeyD) {
		s.currentStep.Add(s.currentStep, big.NewInt(2)) // Move by 2 to stay on odd steps
		// Ensure we stay on odd step
		mod := new(big.Int).Mod(s.currentStep, big.NewInt(2))
		if mod.Sign() == 0 {
			s.currentStep.Add(s.currentStep, big.NewInt(1))
		}
	}
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowLeft) || inpututil.IsKeyJustPressed(ebiten.KeyA) {
		s.currentStep.Sub(s.currentStep, big.NewInt(2)) // Move by 2 to stay on odd steps
		// Ensure we stay on odd step
		mod := new(big.Int).Mod(s.currentStep, big.NewInt(2))
		if mod.Sign() == 0 {
			s.currentStep.Sub(s.currentStep, big.NewInt(1))
		}
	}
	
	// Cache will be automatically invalidated in getCachedStaircase() 
	// by checking if cachedForIndex != currentStep or cachedForCoef != globalCoeficient
	// No need to manually invalidate here
	
	return nil
}

func (s *SpiralStairsChapter) Draw(screen *ebiten.Image) {
	screen.Fill(color.RGBA{30, 30, 40, 255})
	
	centerX := float64(screenWidth) / 2
	centerY := float64(screenHeight) / 2
	
	// Draw the spiral staircase from above
	s.drawSpiralStairs(screen, centerX, centerY)
	
	// Draw the Collatz table
	s.drawCollatzTable(screen)
	
	// Draw shift overlay if shift is pressed
	if ebiten.IsKeyPressed(ebiten.KeyShiftLeft) || ebiten.IsKeyPressed(ebiten.KeyShiftRight) {
		s.drawShiftOverlay(screen)
	}
	
	// Draw jump input overlay if in jump input mode
	if s.jumpInputMode {
		s.drawJumpInput(screen)
	}
	
	// Draw help dialog if open
	if s.showHelp {
		s.drawHelpDialog(screen)
	}
	
	// Draw the current step indicator
	s.drawCurrentStep(screen, centerX, centerY)
}

func (s *SpiralStairsChapter) drawSpiralStairs(screen *ebiten.Image, centerX, centerY float64) {
	// Draw steps around the spiral
	// Show steps from currentStep - 16 to currentStep + 16 (about 2 turns visible)
	// Only draw odd steps
	// Convert to int for the visible range (small window around currentStep)
	currentStepInt := 0
	if s.currentStep.IsInt64() {
		currentStepInt = int(s.currentStep.Int64())
	} else {
		// If too large, just show a small range around 0
		currentStepInt = 0
	}
	
	startStep := currentStepInt - 16
	endStep := currentStepInt + 16
	
	// Ensure start and end are odd
	if startStep%2 == 0 {
		startStep--
	}
	if endStep%2 == 0 {
		endStep++
	}
	
	for step := startStep; step <= endStep; step += 2 {
		// Only draw odd steps
		if step%2 != 0 {
			s.drawStep(screen, centerX, centerY, step, currentStepInt)
		}
	}
}

func (s *SpiralStairsChapter) drawStep(screen *ebiten.Image, centerX, centerY float64, step int, currentStepInt int) {
	// Calculate angle for this step (32 steps per full turn = 2π/32 per step)
	// Since we only show odd steps, this gives us 16 odd steps per turn
	anglePerStep := 2.0 * math.Pi / stepsPerTurn
	baseAngle := float64(step) * anglePerStep
	
	// Add offset to align rightmost and leftmost steps horizontally
	// We want step 1 to be at the rightmost position (angle 0)
	// Step 1 should be at angle 0, so we subtract the angle for step 1
	alignmentOffset := -anglePerStep // Step 1 (first odd step) should be at angle 0
	
	angle := baseAngle + alignmentOffset
	
	// Calculate radius - steps get further from center as we go up
	// Use step number to determine radius (higher steps = larger radius)
	radiusFactor := 1.0 + float64(step-currentStepInt)*0.05
	radius := spiralRadius * radiusFactor * s.zoom
	
	// Calculate step position
	stepX := centerX + radius*math.Cos(angle)
	stepY := centerY + radius*math.Sin(angle)
	
	// Determine if this is the current step
	isCurrent := (step == currentStepInt)
	
	// Draw the step
	stepSize := 20.0 * s.zoom
	if isCurrent {
		// Current step is highlighted
		ebitenutil.DrawRect(screen, stepX-stepSize/2, stepY-stepSize/2, stepSize, stepSize, color.RGBA{255, 200, 100, 255})
		// Draw border
		ebitenutil.DrawRect(screen, stepX-stepSize/2-2, stepY-stepSize/2-2, stepSize+4, 2, color.White)
		ebitenutil.DrawRect(screen, stepX-stepSize/2-2, stepY-stepSize/2-2, 2, stepSize+4, color.White)
		ebitenutil.DrawRect(screen, stepX+stepSize/2, stepY-stepSize/2-2, 2, stepSize+4, color.White)
		ebitenutil.DrawRect(screen, stepX-stepSize/2-2, stepY+stepSize/2, stepSize+4, 2, color.White)
	} else {
		// Other steps
		stepColor := color.RGBA{150, 150, 150, 255}
		if step < currentStepInt {
			// Steps below are darker
			stepColor = color.RGBA{100, 100, 100, 255}
		} else {
			// Steps above are lighter
			stepColor = color.RGBA{180, 180, 180, 255}
		}
		ebitenutil.DrawRect(screen, stepX-stepSize/2, stepY-stepSize/2, stepSize, stepSize, stepColor)
	}
	
	// Draw a line from center to step to show the spiral
	if isCurrent {
		ebitenutil.DrawLine(screen, centerX, centerY, stepX, stepY, color.RGBA{255, 200, 100, 255})
	} else {
		ebitenutil.DrawLine(screen, centerX, centerY, stepX, stepY, color.RGBA{80, 80, 80, 255})
	}
}

func (s *SpiralStairsChapter) getCachedStaircase() ([]StairStep, bool, bool) {
	// Check if cache is valid - compare big.Int values
	cacheValid := false
	if s.cachedForIndex != nil {
		cacheValid = s.cachedForIndex.Cmp(s.currentStep) == 0 && s.cachedForCoef == s.globalCoeficient
	}
	if cacheValid {
		return s.cachedSteps, s.cachedIsUpwards, s.cachedHitLimit
	}
	
	// Cache is invalid, recalculate
	// For spiral staircase, continue up to 50 steps even if direction changes
	maxSteps := 50
	// Convert big.Int to int for CalculateStaircase (it expects int)
	startIndex := 0
	if s.currentStep.IsInt64() {
		startIndex = int(s.currentStep.Int64())
	} else {
		// If too large, use 0 as fallback
		startIndex = 0
	}
	steps, isUpwards, hitLimit := CalculateStaircase(startIndex, s.globalCoeficient, maxSteps, false)
	
	// Update cache
	s.cachedSteps = steps
	s.cachedIsUpwards = isUpwards
	s.cachedHitLimit = hitLimit
	// Create a new big.Int copy for cache key
	s.cachedForIndex = new(big.Int).Set(s.currentStep)
	s.cachedForCoef = s.globalCoeficient
	
	return steps, isUpwards, hitLimit
}

func (s *SpiralStairsChapter) cycleGlobalCoeficient() {
	// Odd numbers from 1 to 21
	oddNumbers := []float64{1, 3, 5, 7, 9, 11, 13, 15, 17, 19, 21}
	
	// Find current index
	currentIndex := -1
	for i, val := range oddNumbers {
		if val == s.globalCoeficient {
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
	s.globalCoeficient = oddNumbers[currentIndex]
}

func (s *SpiralStairsChapter) calculateFrequency(steps []StairStep) *big.Int {
	// Frequency is the product of 2^k for all k values in the staircase steps
	// Use big.Int to handle arbitrarily large values
	frequency := big.NewInt(1)
	
	// Multiply by k values from all steps
	for _, step := range steps {
		// Each k value contributes 2^k to the frequency
		if step.K > 0 {
			// Calculate 2^k using big.Int
			powerOf2 := new(big.Int).Lsh(big.NewInt(1), uint(step.K)) // 1 << k
			frequency.Mul(frequency, powerOf2)
		}
		// If k is 0, we multiply by 1 (no change), so we can skip it
	}
	return frequency
}

func (s *SpiralStairsChapter) drawCollatzTable(screen *ebiten.Image) {
	// Calculate staircase steps (use cache)
	steps, isUpwards, hitLimit := s.getCachedStaircase()
	
	// Calculate frequency
	frequency := s.calculateFrequency(steps)
	
	// Use the utility function to draw the table
	// Convert big.Int to int for StartIndex
	startIndexInt := 0
	if s.currentStep.IsInt64() {
		startIndexInt = int(s.currentStep.Int64())
	}
	data := CollatzTableData{
		Steps:      steps,
		StartIndex: startIndexInt,
		Coefficient: s.globalCoeficient,
		IsUpwards:  isUpwards,
		HitLimit:   hitLimit,
		StopOnDirectionChange: false, // Continue even if direction changes
	}
	DrawCollatzTable(screen, data)
	
	// Draw frequency in the top right corner
	freqText := "Frequency: " + frequency.String()
	freqBounds := text.BoundString(basicfont.Face7x13, freqText)
	freqX := screenWidth - freqBounds.Dx() - 10 // Right-aligned with padding
	freqY := 20 // Top of screen
	text.Draw(screen, freqText, basicfont.Face7x13, freqX, freqY, color.RGBA{200, 150, 255, 255}) // Purple
}

func (s *SpiralStairsChapter) drawShiftOverlay(screen *ebiten.Image) {
	// Draw a small indicator at the bottom when shift is held
	if s.cachedFrequencyJump != nil {
		// Draw a small background box at the bottom
		boxX := 10.0
		boxY := float64(screenHeight) - 50.0
		boxWidth := 200.0
		boxHeight := 40.0
		ebitenutil.DrawRect(screen, boxX, boxY, boxWidth, boxHeight, color.RGBA{0, 100, 200, 150}) // Semi-transparent blue
		
		// Draw frequency jump info
		jumpText := "Jump: " + s.cachedFrequencyJump.String()
		text.Draw(screen, jumpText, basicfont.Face7x13, int(boxX+5), int(boxY+13), color.White)
		
		// Draw instruction
		instText := "Press LEFT/RIGHT to jump"
		text.Draw(screen, instText, basicfont.Face7x13, int(boxX+5), int(boxY+26), color.RGBA{200, 200, 255, 255})
	}
}

func (s *SpiralStairsChapter) drawJumpInput(screen *ebiten.Image) {
	// Draw a small dialog box in the center
	dialogWidth := 300.0
	dialogHeight := 80.0
	dialogX := (float64(screenWidth) - dialogWidth) / 2
	dialogY := (float64(screenHeight) - dialogHeight) / 2
	
	// Draw dialog background
	ebitenutil.DrawRect(screen, dialogX, dialogY, dialogWidth, dialogHeight, color.RGBA{40, 40, 50, 255})
	
	// Draw dialog border
	ebitenutil.DrawRect(screen, dialogX, dialogY, dialogWidth, 2, color.White)
	ebitenutil.DrawRect(screen, dialogX, dialogY, 2, dialogHeight, color.White)
	ebitenutil.DrawRect(screen, dialogX+dialogWidth-2, dialogY, 2, dialogHeight, color.White)
	ebitenutil.DrawRect(screen, dialogX, dialogY+dialogHeight-2, dialogWidth, 2, color.White)
	
	// Draw input prompt
	promptText := "Jump to step: " + s.jumpInputBuffer + "_"
	promptBounds := text.BoundString(basicfont.Face7x13, promptText)
	promptX := int(dialogX + (dialogWidth-float64(promptBounds.Dx()))/2)
	promptY := int(dialogY + 25)
	text.Draw(screen, promptText, basicfont.Face7x13, promptX, promptY, color.White)
	
	// Draw instruction
	instText := "Enter: confirm | ESC: cancel"
	instBounds := text.BoundString(basicfont.Face7x13, instText)
	instX := int(dialogX + (dialogWidth-float64(instBounds.Dx()))/2)
	instY := int(dialogY + 45)
	text.Draw(screen, instText, basicfont.Face7x13, instX, instY, color.Gray{Y: 150})
}

func (s *SpiralStairsChapter) drawHelpDialog(screen *ebiten.Image) {
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
		"MOVEMENT:",
		"  LEFT/RIGHT or A/D    - Move to previous/next odd step",
		"",
		"JUMPING:",
		"  J                    - Open jump dialog to go to specific step",
		"  Shift+LEFT/RIGHT     - Jump by frequency (set with F key)",
		"",
		"FREQUENCY:",
		"  F                    - Calculate and store frequency jump amount",
		"                       - Frequency is product of 2^k for all steps",
		"",
		"COEFFICIENT:",
		"  C                    - Cycle through odd coefficients (1-21)",
		"",
		"RESET:",
		"  R                    - Reset to step 1",
		"",
		"NAVIGATION:",
		"  ESC                  - Return to chapter selection",
		"",
		"JUMP DIALOG:",
		"  Type number          - Enter step number",
		"  Enter               - Confirm and jump",
		"  ESC                 - Cancel",
		"  Backspace            - Delete last character",
		"",
		"HELP:",
		"  H                    - Show/hide this help dialog",
		"  UP/DOWN or W/S       - Scroll help (when open)",
		"",
		"",
		"Press H or ESC to close",
	}
	
	// Draw scrollable content
	startY := int(dialogY) + 50 - s.helpScrollOffset
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
			} else if strings.HasPrefix(line, "  ") {
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
		scrollText := fmt.Sprintf("Scroll: %d/%d", s.helpScrollOffset, totalHeight-int(dialogHeight-70))
		scrollBounds := text.BoundString(basicfont.Face7x13, scrollText)
		scrollX := int(dialogX + dialogWidth - float64(scrollBounds.Dx()) - 10)
		scrollY := int(dialogY + dialogHeight - 20)
		text.Draw(screen, scrollText, basicfont.Face7x13, scrollX, scrollY, color.Gray{Y: 120})
	}
}

func (s *SpiralStairsChapter) IsDialogOpen() bool {
	return s.showHelp || s.jumpInputMode
}

func (s *SpiralStairsChapter) WasEscConsumed() bool {
	return s.escConsumed
}

func (s *SpiralStairsChapter) drawCurrentStep(screen *ebiten.Image, centerX, centerY float64) {
	// Draw info text at the bottom
	// Force recalculation of step text every frame to ensure it updates
	stepText := "Step: " + s.currentStep.String()
	stepBounds := text.BoundString(basicfont.Face7x13, stepText)
	stepX := (screenWidth - stepBounds.Dx()) / 2
	stepY := screenHeight - 30
	text.Draw(screen, stepText, basicfont.Face7x13, stepX, stepY, color.White)
	
	// Draw controls
	controlsText := "Arrow Keys: navigate | Shift+Arrows: jump by frequency | F: update frequency | H: help"
	controlsBounds := text.BoundString(basicfont.Face7x13, controlsText)
	controlsX := (screenWidth - controlsBounds.Dx()) / 2
	controlsY := screenHeight - 15
	text.Draw(screen, controlsText, basicfont.Face7x13, controlsX, controlsY, color.Gray{Y: 100})
}

