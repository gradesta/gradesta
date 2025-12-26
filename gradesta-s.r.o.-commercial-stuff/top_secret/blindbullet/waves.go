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

// WaveInfo represents a cosine wave
type WaveInfo struct {
	Center   int     // Center position on the number line
	Period   float64 // Period of the wave
	WaveNum  int     // Wave number (for label)
	Label    string  // Label to display at peaks
}

// WavesChapter implements the Waves chapter
type WavesChapter struct {
	currentIndex    *big.Int // Current position on the number line
	globalCoeficient float64  // Coefficient for Collatz calculations
	escConsumed     bool     // Whether ESC was consumed by a dialog this frame
	cachedSteps     map[string][]StairStep // Cache for Collatz steps
	cachedForIndex  *big.Int // Index for which steps are cached
	cachedForCoef   float64  // Coefficient for which steps are cached
}

// NewWavesChapter creates a new Waves chapter
func NewWavesChapter() *WavesChapter {
	return &WavesChapter{
		currentIndex:    big.NewInt(-1),
		globalCoeficient: 3.0,
		cachedSteps:     make(map[string][]StairStep),
	}
}

// generateWaves generates the cosine waves based on the Collatz sequence pattern
// Starting from -1, each wave's center follows the pattern:
// Wave 1: center = -1, period = 4
// Wave 2: center = 1, period = 8
// Wave 3: center = -3, period = 16
// Wave 4: center = 5, period = 32
// etc.
func (w *WavesChapter) generateWaves(maxWaves int) []WaveInfo {
	waves := []WaveInfo{}
	
	// Start with wave 1: center = -1, period = 4
	center := -1
	period := 4.0
	waveNum := 1
	
	for len(waves) < maxWaves {
		waves = append(waves, WaveInfo{
			Center:  center,
			Period:  period,
			WaveNum: waveNum,
			Label:   strconv.Itoa(waveNum), // Convert 1->"1", 2->"2", etc.
		})
		
		// Calculate next wave
		// Pattern: alternate between adding and subtracting half the current period
		halfPeriod := int(period / 2)
		if waveNum%2 == 1 {
			// Odd wave number: add half period to get next center
			center = center + halfPeriod
		} else {
			// Even wave number: subtract half period to get next center
			center = center - halfPeriod
		}
		
		waveNum++
		period *= 2.0 // Period doubles each time
	}
	
	return waves
}

// Update updates the Waves chapter
func (w *WavesChapter) Update() error {
	w.escConsumed = false // Reset at start of frame
	
	// Check if shift is pressed first
	shiftPressed := ebiten.IsKeyPressed(ebiten.KeyShiftLeft) || ebiten.IsKeyPressed(ebiten.KeyShiftRight)
	
	// Handle left/right arrow keys to move by 2 steps (only if shift is NOT pressed)
	if !shiftPressed {
		if inpututil.IsKeyJustPressed(ebiten.KeyArrowLeft) || inpututil.IsKeyJustPressed(ebiten.KeyA) {
			// Move left by 2
			w.currentIndex.Sub(w.currentIndex, big.NewInt(2))
		}
		
		if inpututil.IsKeyJustPressed(ebiten.KeyArrowRight) || inpututil.IsKeyJustPressed(ebiten.KeyD) {
			// Move right by 2
			w.currentIndex.Add(w.currentIndex, big.NewInt(2))
		}
	}
	
	// Handle Shift+arrow keys to move to next peak of the wave that peaks at current position
	if shiftPressed && (inpututil.IsKeyJustPressed(ebiten.KeyArrowLeft) || inpututil.IsKeyJustPressed(ebiten.KeyA) || 
		inpututil.IsKeyJustPressed(ebiten.KeyArrowRight) || inpututil.IsKeyJustPressed(ebiten.KeyD)) {
		// Find the wave that has a peak at the current index
		// If multiple waves peak at the same position, use the one with the smallest period
		waves := w.generateWaves(20) // Generate enough waves
		currentInt := w.currentIndex.Int64()
		
		var bestWave *WaveInfo
		bestPeriod := math.MaxFloat64
		
		for i := range waves {
			wave := &waves[i]
			// Check if current index is at a peak of this wave
			// Peaks occur when (index - center) is a multiple of period
			diff := currentInt - int64(wave.Center)
			// Check if diff is a multiple of period using integer arithmetic
			// Use absolute value for modulo to handle negative numbers correctly
			periodInt := int64(wave.Period)
			if periodInt > 0 {
				diffAbs := diff
				if diffAbs < 0 {
					diffAbs = -diffAbs
				}
				if diffAbs%periodInt == 0 {
					// This wave has a peak at the current position
					// Use the wave with the smallest period (most specific)
					if wave.Period < bestPeriod {
						bestWave = wave
						bestPeriod = wave.Period
					}
				}
			}
		}
		
		// If we found a wave, move by its period
		if bestWave != nil {
			periodBig := big.NewInt(int64(bestWave.Period))
			if inpututil.IsKeyJustPressed(ebiten.KeyArrowLeft) || inpututil.IsKeyJustPressed(ebiten.KeyA) {
				// Move to previous peak
				w.currentIndex.Sub(w.currentIndex, periodBig)
			}
			if inpututil.IsKeyJustPressed(ebiten.KeyArrowRight) || inpututil.IsKeyJustPressed(ebiten.KeyD) {
				// Move to next peak
				w.currentIndex.Add(w.currentIndex, periodBig)
			}
		}
	}
	
	// Handle 'r' key to reset to -1
	if inpututil.IsKeyJustPressed(ebiten.KeyR) {
		w.currentIndex.SetInt64(-1)
		// Invalidate cache
		w.cachedSteps = make(map[string][]StairStep)
		w.cachedForIndex = nil
	}
	
	// Handle 'c' key to cycle coefficient
	if inpututil.IsKeyJustPressed(ebiten.KeyC) {
		w.cycleGlobalCoeficient()
	}
	
	// ESC is handled by main.go to return to launch screen
	// We don't set escConsumed here since we don't have dialogs
	
	return nil
}

// cycleGlobalCoeficient cycles through odd coefficients 1-21
func (w *WavesChapter) cycleGlobalCoeficient() {
	oddNumbers := []float64{1, 3, 5, 7, 9, 11, 13, 15, 17, 19, 21}
	currentIndex := 0
	for i, val := range oddNumbers {
		if val == w.globalCoeficient {
			currentIndex = i
			break
		}
	}
	currentIndex = (currentIndex + 1) % len(oddNumbers)
	w.globalCoeficient = oddNumbers[currentIndex]
	// Invalidate cache
	w.cachedSteps = make(map[string][]StairStep)
	w.cachedForIndex = nil
}

// getCachedStaircase gets or calculates the staircase for the current index
func (w *WavesChapter) getCachedStaircase() ([]StairStep, bool, bool) {
	// Check if we have a cached result
	if w.cachedForIndex != nil && w.cachedForIndex.Cmp(w.currentIndex) == 0 &&
		w.cachedForCoef == w.globalCoeficient {
		// Return cached result
		cacheKey := w.currentIndex.String() + "_" + string(rune(int(w.globalCoeficient)))
		if steps, ok := w.cachedSteps[cacheKey]; ok {
			return steps, false, false // We don't track isUpwards/hitLimit in cache, use defaults
		}
	}
	
	// Calculate new staircase
	steps, isUpwards, hitLimit := CalculateStaircase(w.currentIndex, w.globalCoeficient, 50, false)
	
	// Cache the result
	w.cachedForIndex = new(big.Int).Set(w.currentIndex)
	w.cachedForCoef = w.globalCoeficient
	cacheKey := w.currentIndex.String() + "_" + string(rune(int(w.globalCoeficient)))
	w.cachedSteps[cacheKey] = steps
	
	return steps, isUpwards, hitLimit
}

// Draw draws the Waves chapter
func (w *WavesChapter) Draw(screen *ebiten.Image) {
	// Fill background
	screen.Fill(color.RGBA{20, 20, 30, 255})
	
	// Draw title
	titleText := "Waves"
	titleBounds := text.BoundString(basicfont.Face7x13, titleText)
	titleX := (screenWidth - titleBounds.Dx()) / 2
	titleY := 20
	text.Draw(screen, titleText, basicfont.Face7x13, titleX, titleY, color.White)
	
	// Generate waves (generate enough to cover visible area)
	waves := w.generateWaves(20)
	
	// Calculate visible range (show about 100 units on each side of current index)
	currentInt := w.currentIndex.Int64()
	visibleStart := float64(currentInt) - 50
	visibleEnd := float64(currentInt) + 50
	
	// Draw number line (positioned lower to use more of the screen)
	lineY := screenHeight * 3 / 4
	lineStartX := 50.0
	lineEndX := float64(screenWidth - 50)
	lineLength := lineEndX - lineStartX
	
	// Draw horizontal number line
	ebitenutil.DrawLine(screen, lineStartX, float64(lineY), lineEndX, float64(lineY), color.White)
	
	// Draw tick marks and numbers on the number line
	for i := visibleStart; i <= visibleEnd; i += 2 {
		if int(i)%10 == 0 { // Only show every 10th number to avoid clutter
			x := lineStartX + (float64(i)-visibleStart)*(lineLength/100.0)
			if x >= lineStartX && x <= lineEndX {
				// Draw tick mark
				ebitenutil.DrawLine(screen, x, float64(lineY-5), x, float64(lineY+5), color.White)
				// Draw number
				numText := big.NewInt(int64(i)).String()
				numBounds := text.BoundString(basicfont.Face7x13, numText)
				text.Draw(screen, numText, basicfont.Face7x13, int(x)-numBounds.Dx()/2, lineY+20, color.White)
			}
		}
	}
	
	// Draw current index marker
	currentX := lineStartX + (float64(currentInt)-visibleStart)*(lineLength/100.0)
	if currentX >= lineStartX && currentX <= lineEndX {
		ebitenutil.DrawRect(screen, currentX-2, float64(lineY-10), 4, 20, color.RGBA{255, 255, 0, 255})
	}
	
	// Draw cosine waves
	waveAmplitude := 30.0 // Height of waves
	waveSpacing := 40.0    // Vertical spacing between waves
	
	for waveIdx, wave := range waves {
		waveY := float64(lineY) - float64(waveIdx+1)*waveSpacing
		
		// Draw cosine wave
		points := []struct{ x, y float64 }{}
		for x := visibleStart; x <= visibleEnd; x += 0.5 {
			// Calculate y position based on cosine
			// Cosine centered at wave.Center with period wave.Period
			// Negate cosine so peaks are at the top (valleys at bottom)
			phase := 2.0 * math.Pi * (x - float64(wave.Center)) / wave.Period
			y := waveY - math.Cos(phase)*waveAmplitude
			
			screenX := lineStartX + (x-visibleStart)*(lineLength/100.0)
			if screenX >= lineStartX && screenX <= lineEndX {
				points = append(points, struct{ x, y float64 }{screenX, y})
			}
		}
		
		// Draw the wave as connected lines
		for i := 0; i < len(points)-1; i++ {
			ebitenutil.DrawLine(screen, points[i].x, points[i].y, points[i+1].x, points[i+1].y, color.RGBA{100, 200, 255, 255})
		}
		
		// Draw labels at peaks
		// Find peaks (where cosine = 1, i.e., phase is multiple of 2π)
		// Peaks occur when (x - center) is a multiple of period
		startPeak := math.Ceil((visibleStart - float64(wave.Center)) / wave.Period) * wave.Period
		for x := float64(wave.Center) + startPeak; x <= visibleEnd; x += wave.Period {
			screenX := lineStartX + (x-visibleStart)*(lineLength/100.0)
			if screenX >= lineStartX && screenX <= lineEndX {
				peakY := waveY - waveAmplitude
				labelBounds := text.BoundString(basicfont.Face7x13, wave.Label)
				text.Draw(screen, wave.Label, basicfont.Face7x13, int(screenX)-labelBounds.Dx()/2, int(peakY)-10, color.RGBA{255, 255, 100, 255})
			}
		}
	}
	
	// Draw Collatz steps table (always draw, even if no steps - DrawCollatzTableAt handles empty case)
	steps, _, _ := w.getCachedStaircase()
	tableData := CollatzTableData{
		Steps:                steps,
		StartIndex:           w.currentIndex,
		Coefficient:          w.globalCoeficient,
		StopOnDirectionChange: false,
	}
	DrawCollatzTableAt(screen, tableData, 10, 70)
	
	// Draw current index and coefficient
	infoText := "Index: " + w.currentIndex.String() + " | Coefficient: " + strconv.FormatFloat(w.globalCoeficient, 'f', -1, 64)
	infoBounds := text.BoundString(basicfont.Face7x13, infoText)
	infoX := (screenWidth - infoBounds.Dx()) / 2
	infoY := screenHeight - 30
	text.Draw(screen, infoText, basicfont.Face7x13, infoX, infoY, color.White)
	
	// Draw controls
	controlsText := "LEFT/RIGHT: move by 2 | Shift+LEFT/RIGHT: move by wave period | C: cycle coefficient | R: reset to -1"
	controlsBounds := text.BoundString(basicfont.Face7x13, controlsText)
	controlsX := (screenWidth - controlsBounds.Dx()) / 2
	controlsY := screenHeight - 15
	text.Draw(screen, controlsText, basicfont.Face7x13, controlsX, controlsY, color.Gray{Y: 150})
}

// WasEscConsumed returns whether ESC was consumed by a dialog this frame
func (w *WavesChapter) WasEscConsumed() bool {
	return w.escConsumed
}
