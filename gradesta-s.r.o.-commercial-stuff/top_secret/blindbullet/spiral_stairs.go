package main

import (
	"image/color"
	"math"
	"math/big"

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
	cachedFrequencyJump *big.Int // Jump amount (frequency * 2) cached when shift is first pressed
	shiftWasPressed     bool     // Track if shift was just pressed (to recalculate frequency)
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
	// Handle 'c' key to cycle through global coefficient values (odd numbers 1-21)
	if inpututil.IsKeyJustPressed(ebiten.KeyC) {
		s.cycleGlobalCoeficient()
	}
	
	// Handle 'r' key to reset to step 1
	if inpututil.IsKeyJustPressed(ebiten.KeyR) {
		s.currentStep.SetInt64(1)
		return nil
	}
	
	// Handle shift+arrow keys to jump by frequency
	shiftPressed := ebiten.IsKeyPressed(ebiten.KeyShiftLeft) || ebiten.IsKeyPressed(ebiten.KeyShiftRight)
	shiftJustPressed := inpututil.IsKeyJustPressed(ebiten.KeyShiftLeft) || inpututil.IsKeyJustPressed(ebiten.KeyShiftRight)
	
	if shiftPressed {
		// If shift was just pressed, recalculate frequency
		if shiftJustPressed || !s.shiftWasPressed {
			steps, _, _ := s.getCachedStaircase()
			frequency := s.calculateFrequency(steps)
			// Multiply by 2 to stay on odd steps
			s.cachedFrequencyJump = new(big.Int).Mul(frequency, big.NewInt(2))
			s.shiftWasPressed = true
		}
		
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
	} else {
		// Shift is not pressed, reset the flag
		s.shiftWasPressed = false
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
	// Frequency is the product of 2^k for each k value
	// Include the starting point's k value
	// Use big.Int to handle arbitrarily large values
	frequency := big.NewInt(1)
	
	// Calculate k for the starting point
	// Convert big.Int to float64 for calculation
	currentStepFloat := 0.0
	if s.currentStep.IsInt64() {
		currentStepFloat = float64(s.currentStep.Int64())
	} else {
		// For very large numbers, use a big.Float conversion
		bigFloat := new(big.Float).SetInt(s.currentStep)
		currentStepFloat, _ = bigFloat.Float64()
	}
	startYValue := s.globalCoeficient*currentStepFloat + 1.0
	startYValueInt := int(startYValue)
	startK := findLargestPowerOf2(startYValueInt)
	if startK > 0 {
		// Calculate 2^k using big.Int
		powerOf2 := new(big.Int).Lsh(big.NewInt(1), uint(startK)) // 1 << k
		frequency.Mul(frequency, powerOf2)
	}
	
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

func (s *SpiralStairsChapter) drawCurrentStep(screen *ebiten.Image, centerX, centerY float64) {
	// Draw info text at the bottom
	// Force recalculation of step text every frame to ensure it updates
	stepText := "Step: " + s.currentStep.String()
	stepBounds := text.BoundString(basicfont.Face7x13, stepText)
	stepX := (screenWidth - stepBounds.Dx()) / 2
	stepY := screenHeight - 30
	text.Draw(screen, stepText, basicfont.Face7x13, stepX, stepY, color.White)
	
	// Draw controls
	controlsText := "LEFT/RIGHT: move | Shift+LEFT/RIGHT: jump by frequency | C: change coefficient | R: reset to step 1 | ESC: back"
	controlsBounds := text.BoundString(basicfont.Face7x13, controlsText)
	controlsX := (screenWidth - controlsBounds.Dx()) / 2
	controlsY := screenHeight - 15
	text.Draw(screen, controlsText, basicfont.Face7x13, controlsX, controlsY, color.Gray{Y: 100})
}

