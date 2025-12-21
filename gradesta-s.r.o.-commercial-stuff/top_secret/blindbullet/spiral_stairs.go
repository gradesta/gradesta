package main

import (
	"image/color"
	"math"
	"strconv"

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
	currentStep      int     // Current step (0-based, can be negative)
	zoom             float64 // Zoom level
	globalCoeficient float64 // Coefficient for the line equation (y = globalCoeficient*x + 1)
	
	// Cached staircase calculation
	cachedSteps      []StairStep
	cachedIsUpwards  bool
	cachedHitLimit   bool
	cachedForIndex   int
	cachedForCoef    float64
}

// NewSpiralStairsChapter creates a new Spiral Staircase chapter
func NewSpiralStairsChapter() *SpiralStairsChapter {
	return &SpiralStairsChapter{
		currentStep:      1,   // Start at an odd step
		zoom:             1.0,
		globalCoeficient: 3.0, // y = 3x + 1
		// Cache will be invalid initially since cachedForIndex (0) != currentStep (1)
	}
}

func (s *SpiralStairsChapter) Update() error {
	// Handle 'c' key to cycle through global coefficient values (odd numbers 1-21)
	if inpututil.IsKeyJustPressed(ebiten.KeyC) {
		s.cycleGlobalCoeficient()
	}
	
	// Handle left/right movement - only allow odd steps
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowRight) || inpututil.IsKeyJustPressed(ebiten.KeyD) {
		s.currentStep += 2 // Move by 2 to stay on odd steps
		// Ensure we stay on odd step
		if s.currentStep%2 == 0 {
			s.currentStep++
		}
	}
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowLeft) || inpututil.IsKeyJustPressed(ebiten.KeyA) {
		s.currentStep -= 2 // Move by 2 to stay on odd steps
		// Ensure we stay on odd step
		if s.currentStep%2 == 0 {
			s.currentStep--
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
	
	// Draw the current step indicator
	s.drawCurrentStep(screen, centerX, centerY)
}

func (s *SpiralStairsChapter) drawSpiralStairs(screen *ebiten.Image, centerX, centerY float64) {
	// Draw steps around the spiral
	// Show steps from currentStep - 16 to currentStep + 16 (about 2 turns visible)
	// Only draw odd steps
	startStep := s.currentStep - 16
	endStep := s.currentStep + 16
	
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
			s.drawStep(screen, centerX, centerY, step)
		}
	}
}

func (s *SpiralStairsChapter) drawStep(screen *ebiten.Image, centerX, centerY float64, step int) {
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
	radiusFactor := 1.0 + float64(step-s.currentStep)*0.05
	radius := spiralRadius * radiusFactor * s.zoom
	
	// Calculate step position
	stepX := centerX + radius*math.Cos(angle)
	stepY := centerY + radius*math.Sin(angle)
	
	// Determine if this is the current step
	isCurrent := (step == s.currentStep)
	
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
		if step < s.currentStep {
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
	// Check if cache is valid
	if s.cachedForIndex == s.currentStep && s.cachedForCoef == s.globalCoeficient {
		return s.cachedSteps, s.cachedIsUpwards, s.cachedHitLimit
	}
	
	// Cache is invalid, recalculate
	// For spiral staircase, continue up to 50 steps even if direction changes
	maxSteps := 50
	steps, isUpwards, hitLimit := CalculateStaircase(s.currentStep, s.globalCoeficient, maxSteps, false)
	
	// Update cache
	s.cachedSteps = steps
	s.cachedIsUpwards = isUpwards
	s.cachedHitLimit = hitLimit
	s.cachedForIndex = s.currentStep
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

func (s *SpiralStairsChapter) drawCollatzTable(screen *ebiten.Image) {
	// Calculate staircase steps (use cache)
	steps, isUpwards, hitLimit := s.getCachedStaircase()
	
	// Use the utility function to draw the table
	data := CollatzTableData{
		Steps:      steps,
		StartIndex: s.currentStep,
		Coefficient: s.globalCoeficient,
		IsUpwards:  isUpwards,
		HitLimit:   hitLimit,
		StopOnDirectionChange: false, // Continue even if direction changes
	}
	DrawCollatzTable(screen, data)
}

func (s *SpiralStairsChapter) drawCurrentStep(screen *ebiten.Image, centerX, centerY float64) {
	// Draw info text at the bottom
	// Force recalculation of step text every frame to ensure it updates
	stepText := "Step: " + strconv.Itoa(s.currentStep)
	stepBounds := text.BoundString(basicfont.Face7x13, stepText)
	stepX := (screenWidth - stepBounds.Dx()) / 2
	stepY := screenHeight - 30
	text.Draw(screen, stepText, basicfont.Face7x13, stepX, stepY, color.White)
	
	// Draw controls
	controlsText := "LEFT/RIGHT: move | C: change coefficient | ESC: back"
	controlsBounds := text.BoundString(basicfont.Face7x13, controlsText)
	controlsX := (screenWidth - controlsBounds.Dx()) / 2
	controlsY := screenHeight - 15
	text.Draw(screen, controlsText, basicfont.Face7x13, controlsX, controlsY, color.Gray{Y: 100})
}

