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

// WaveHistoryEntry stores information about a wave we've up-arrowed through
type WaveHistoryEntry struct {
	WaveNum int     // Wave number
	Center  int     // Center of the wave
	Period  float64 // Period of the wave
}

// WavesChapter implements the Waves chapter
type WavesChapter struct {
	currentIndex    *big.Int // Current position on the number line
	globalCoeficient float64  // Coefficient for Collatz calculations
	escConsumed     bool     // Whether ESC was consumed by a dialog this frame
	cachedSteps     map[string][]StairStep // Cache for Collatz steps
	cachedForIndex  *big.Int // Index for which steps are cached
	cachedForCoef   float64  // Coefficient for which steps are cached
	upArrowHistory  []WaveHistoryEntry // History of waves we've up-arrowed through
}

// NewWavesChapter creates a new Waves chapter
func NewWavesChapter() *WavesChapter {
	return &WavesChapter{
		currentIndex:    big.NewInt(-1),
		globalCoeficient: 3.0,
		cachedSteps:     make(map[string][]StairStep),
		upArrowHistory:  []WaveHistoryEntry{},
	}
}

// generateWaves generates the cosine waves aligned with k values from the Collatz steps table
// For base layer: uses findWaveCenterForK to find centers
// For higher layers: iterates number line and adds waves based on k values from steps table
func (w *WavesChapter) generateWaves(maxWaves int, periodMultiplier float64) []WaveInfo {
	waves := []WaveInfo{}
	
	// For base layer (no history), use the original approach
	if len(w.upArrowHistory) == 0 {
		period := 4.0
		waveNum := 1
		
		for len(waves) < maxWaves {
			// Find center by locating an index where k = waveNum
			center := findWaveCenterForK(waveNum, w.globalCoeficient)
			
			waves = append(waves, WaveInfo{
				Center:  center,
				Period:  period * periodMultiplier, // Apply period multiplier
				WaveNum: waveNum,
				Label:   strconv.Itoa(waveNum), // Convert 1->"1", 2->"2", etc.
			})
			
			waveNum++
			period *= 2.0 // Period doubles each time
		}
		
		return waves
	}
	
	// For higher layers: iterate number line and add waves based on k values
	// Get the current index and calculate visible range
	currentInt := w.currentIndex.Int64()
	layerPeriod := w.getCurrentLayerPeriod()
	visibleRange := layerPeriod * 20 // Check a reasonable range
	historyLen := len(w.upArrowHistory)
	
	// Track which k values we've already added waves for
	addedKValues := make(map[int]bool)
	
	// First, ensure we generate a wave for the k value at the current index
	// This ensures the test can always find a wave for the current position
	currentSteps, _, _ := CalculateStaircase(w.currentIndex, w.globalCoeficient, 50, false)
	if historyLen < len(currentSteps) {
		currentK := currentSteps[historyLen].K
		if !addedKValues[currentK] {
			// Calculate frequency for current index
			frequency := big.NewInt(1)
			for i := 0; i <= historyLen && i < len(currentSteps); i++ {
				if currentSteps[i].K > 0 {
					powerOf2 := new(big.Int).Lsh(big.NewInt(1), uint(currentSteps[i].K))
					frequency.Mul(frequency, powerOf2)
				}
			}
			frequencyFloat := new(big.Float).SetInt(frequency)
			freqFloat64, _ := frequencyFloat.Float64()
			period := 2.0 * freqFloat64
			if period < 4.0 {
				period = 4.0
			}
			waves = append(waves, WaveInfo{
				Center:  int(currentInt),
				Period:  period,
				WaveNum: currentK,
				Label:   strconv.Itoa(currentK),
			})
			addedKValues[currentK] = true
		}
	}
	
	// Iterate from left to right on the number line
	// Check positions spaced by layerPeriod (the points on the number line)
	startPos := currentInt - int64(visibleRange)
	endPos := currentInt + int64(visibleRange)
	
	// Align startPos to the nearest point on the number line
	if len(w.upArrowHistory) > 0 {
		layerCenter := w.getCurrentLayerCenter()
		// Round to nearest multiple of layerPeriod from layerCenter
		offset := float64(startPos - int64(layerCenter))
		alignedOffset := math.Round(offset/layerPeriod) * layerPeriod
		startPos = int64(layerCenter) + int64(alignedOffset)
	} else {
		// Base layer: ensure odd numbers
		if startPos%2 == 0 {
			startPos++
		}
	}
	
	// Step by the layer period to check each point on the number line
	stepSize := int64(layerPeriod)
	if stepSize < 1 {
		stepSize = 1
	}
	for pos := startPos; pos <= endPos && len(waves) < maxWaves; pos += stepSize {
		// Check if any existing wave peaks at this position
		hasWave := false
		for _, wave := range waves {
			diff := pos - int64(wave.Center)
			periodInt := int64(wave.Period)
			if periodInt > 0 {
				diffAbs := diff
				if diffAbs < 0 {
					diffAbs = -diffAbs
				}
				if diffAbs%periodInt == 0 {
					hasWave = true
					break
				}
			}
		}
		
		if hasWave {
			continue // Already have a wave peaking here
		}
		
		// Get the steps table for this position
		posBig := big.NewInt(pos)
		steps, _, _ := CalculateStaircase(posBig, w.globalCoeficient, 50, false)
		
		// Find the "next k value" using the same logic as up arrow
		// Row index = history length
		var kValue int
		if historyLen < len(steps) {
			kValue = steps[historyLen].K
		} else {
			// If we're beyond the steps, calculate k directly from index
			coefBig := big.NewInt(int64(w.globalCoeficient))
			yValueBig := new(big.Int).Mul(coefBig, posBig)
			yValueBig.Add(yValueBig, big.NewInt(1))
			kValue = findLargestPowerOf2Big(yValueBig)
		}
		
		// Skip if we've already added a wave for this k value
		if addedKValues[kValue] {
			continue
		}
		
		// Calculate period using frequency calculation
		// Frequency is product of 2^k for all k values in steps up to and including historyLen
		// This matches the frequency calculation used in other chapters
		frequency := big.NewInt(1)
		for i := 0; i <= historyLen && i < len(steps); i++ {
			if steps[i].K > 0 {
				powerOf2 := new(big.Int).Lsh(big.NewInt(1), uint(steps[i].K)) // 1 << k
				frequency.Mul(frequency, powerOf2)
			}
		}
		
		// Convert frequency to float64 for period calculation
		// The period should be based on the frequency
		// Period = 2 * frequency (since period is 2x the spacing between peaks)
		frequencyFloat := new(big.Float).SetInt(frequency)
		freqFloat64, _ := frequencyFloat.Float64()
		period := 2.0 * freqFloat64
		
		// Ensure minimum period (at least 4.0)
		if period < 4.0 {
			period = 4.0
		}
		
		// Add wave with this k value
		waves = append(waves, WaveInfo{
			Center:  int(pos),
			Period:  period,
			WaveNum: kValue,
			Label:   strconv.Itoa(kValue),
		})
		
		addedKValues[kValue] = true
	}
	
	return waves
}

// calculatePeriodMultiplier calculates the cumulative period multiplier from upArrowHistory
// The multiplier is the product of all base periods (not multiplied periods) in history
// We divide by 2 for each entry to get the correct scaling (period is 2x the spacing between peaks)
func (w *WavesChapter) calculatePeriodMultiplier() float64 {
	if len(w.upArrowHistory) == 0 {
		return 1.0
	}
	
	multiplier := 1.0
	for _, entry := range w.upArrowHistory {
		// entry.Period is the base period (from when we up-arrowed)
		// Divide by 2 to get the correct multiplier (since period is 2x the spacing)
		multiplier *= entry.Period / 2.0
	}
	
	return multiplier
}

// getCurrentLayerCenter returns the center of the current layer (last wave in history, or 0 if none)
func (w *WavesChapter) getCurrentLayerCenter() int {
	if len(w.upArrowHistory) == 0 {
		return 0 // Base layer, no specific center
	}
	return w.upArrowHistory[len(w.upArrowHistory)-1].Center
}

// getCurrentLayerPeriod returns the period spacing for the number line
// Base layer: spacing of 2 (odd numbers)
// Other layers: frequency * 2, where frequency = product of 2^k for each k value in history
// We multiply by 2 because we're in an odd-only sphere (spacing between odd numbers is 2)
// Example: history [1, 1] -> (2^1 * 2^1) * 2 = (2 * 2) * 2 = 8
// Example: history [3, 3] -> (2^3 * 2^3) * 2 = (8 * 8) * 2 = 128
func (w *WavesChapter) getCurrentLayerPeriod() float64 {
	if len(w.upArrowHistory) == 0 {
		// Base layer: spacing of 2 (odd numbers)
		return 2.0
	}
	
	// Calculate frequency as product of 2^k for each wave number in history
	frequency := 1.0
	for _, entry := range w.upArrowHistory {
		// Each wave number k contributes 2^k to the frequency
		powerOf2 := math.Pow(2.0, float64(entry.WaveNum))
		frequency *= powerOf2
	}
	// Multiply by 2 because we're in an odd-only sphere
	return frequency * 2.0
}

// Update updates the Waves chapter
func (w *WavesChapter) Update() error {
	w.escConsumed = false // Reset at start of frame
	
	// Check if shift is pressed first
	shiftPressed := ebiten.IsKeyPressed(ebiten.KeyShiftLeft) || ebiten.IsKeyPressed(ebiten.KeyShiftRight)
	
	// Handle left/right arrow keys to move to the next point on the number line (only if shift is NOT pressed)
	if !shiftPressed {
		// Move by the spacing of points on the number line
		// Base layer: spacing is 2 (odd numbers), so move by 2 to get to next point
		// Other layers: move by the product of all periods in history (like frequency)
		moveAmount := w.getCurrentLayerPeriod() // This calculates the product for other layers
		
		periodBig := big.NewInt(int64(moveAmount))
		
		if inpututil.IsKeyJustPressed(ebiten.KeyArrowLeft) || inpututil.IsKeyJustPressed(ebiten.KeyA) {
			// Move left by layer period
			w.currentIndex.Sub(w.currentIndex, periodBig)
		}
		
		if inpututil.IsKeyJustPressed(ebiten.KeyArrowRight) || inpututil.IsKeyJustPressed(ebiten.KeyD) {
			// Move right by layer period
			w.currentIndex.Add(w.currentIndex, periodBig)
		}
	}
	
	// Handle Shift+arrow keys to move to next peak of the wave that peaks at current position
	if shiftPressed && (inpututil.IsKeyJustPressed(ebiten.KeyArrowLeft) || inpututil.IsKeyJustPressed(ebiten.KeyA) || 
		inpututil.IsKeyJustPressed(ebiten.KeyArrowRight) || inpututil.IsKeyJustPressed(ebiten.KeyD)) {
		// Find the wave that has a peak at the current index
		// If multiple waves peak at the same position, use the one with the smallest period
		periodMultiplier := w.calculatePeriodMultiplier()
		waves := w.generateWaves(20, periodMultiplier) // Generate enough waves
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
		w.upArrowHistory = []WaveHistoryEntry{} // Reset up arrow history
		// Invalidate cache
		w.cachedSteps = make(map[string][]StairStep)
		w.cachedForIndex = nil
	}
	
	// Handle Up arrow to create new visualization layer
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowUp) || inpututil.IsKeyJustPressed(ebiten.KeyW) {
		// Get the steps table for the current index
		steps, _, _ := w.getCachedStaircase()
		
		// The k value should be from row index equal to history length
		// Row index = history length (0-indexed)
		historyLen := len(w.upArrowHistory)
		
		var kValue int
		if historyLen < len(steps) {
			// Get k value from the steps table at row index = history length
			kValue = steps[historyLen].K
		} else {
			// If we're beyond the steps, calculate k directly from index
			coefBig := big.NewInt(int64(w.globalCoeficient))
			yValueBig := new(big.Int).Mul(coefBig, w.currentIndex)
			yValueBig.Add(yValueBig, big.NewInt(1))
			kValue = findLargestPowerOf2Big(yValueBig)
		}
		
		// Find the wave info for this k value
		// Generate base waves to find the wave with this k value
		waves := w.generateWaves(20, 1.0)
		var targetWave *WaveInfo
		for i := range waves {
			if waves[i].WaveNum == kValue {
				targetWave = &waves[i]
				break
			}
		}
		
		// If we found a wave, add it to the history
		if targetWave != nil {
			w.upArrowHistory = append(w.upArrowHistory, WaveHistoryEntry{
				WaveNum: targetWave.WaveNum,
				Center:  targetWave.Center,
				Period:  targetWave.Period, // This is the base period (not multiplied)
			})
		}
	}
	
	// Handle Down arrow to go back down a level
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowDown) || inpututil.IsKeyJustPressed(ebiten.KeyS) {
		if len(w.upArrowHistory) > 0 {
			// Remove the last entry
			w.upArrowHistory = w.upArrowHistory[:len(w.upArrowHistory)-1]
		}
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
	
	// Draw up arrow history list
	if len(w.upArrowHistory) > 0 {
		historyText := "History: "
		for i, entry := range w.upArrowHistory {
			if i > 0 {
				historyText += ", "
			}
			historyText += strconv.Itoa(entry.WaveNum)
		}
		historyX := 10
		historyY := 35
		text.Draw(screen, historyText, basicfont.Face7x13, historyX, historyY, color.RGBA{200, 200, 255, 255})
	}
	
	// Calculate period multiplier from up arrow history
	periodMultiplier := w.calculatePeriodMultiplier()
	
	// Get current layer center and period for number line
	layerCenter := w.getCurrentLayerCenter()
	// Use the product of all periods in history for number line spacing (like frequency)
	// Base layer: spacing of 2 (odd numbers: -1, 1, 3, 5, ...)
	// Other layers: spacing equals the product of all periods in history
	layerPeriod := w.getCurrentLayerPeriod() // This calculates the product for other layers
	
	// Generate waves (generate enough to cover visible area)
	waves := w.generateWaves(20, periodMultiplier)
	
	// Calculate visible range centered on the current index
	// Show about 10 periods on each side (based on multiplied layer period)
	currentInt := w.currentIndex.Int64()
	visibleRange := layerPeriod * 10
	visibleStart := float64(currentInt) - visibleRange
	visibleEnd := float64(currentInt) + visibleRange
	
	// Draw number line (positioned lower to use more of the screen)
	lineY := screenHeight * 3 / 4
	lineStartX := 50.0
	lineEndX := float64(screenWidth - 50)
	lineLength := lineEndX - lineStartX
	
	// Draw horizontal number line
	ebitenutil.DrawLine(screen, lineStartX, float64(lineY), lineEndX, float64(lineY), color.White)
	
	// Draw points and numbers on the number line
	// Base layer: points at odd numbers (-1, 1, 3, 5, ...) with spacing of 2
	// Other layers: points spaced by the period of the wave we up-arrowed through
	var startPoint float64
	if len(w.upArrowHistory) == 0 {
		// Base layer: show odd numbers (start from nearest odd number)
		// Ensure we start at an odd number
		startPoint = math.Ceil((visibleStart-1)/2) * 2 + 1
	} else {
		// Other layers: show multiples of period, centered on layer center
		startPoint = math.Ceil((visibleStart-float64(layerCenter))/layerPeriod) * layerPeriod + float64(layerCenter)
	}
	// Draw points spaced by layerPeriod (2.0 for base layer, stored period for other layers)
	for x := startPoint; x <= visibleEnd; x += layerPeriod {
		screenX := lineStartX + (x-visibleStart)*(lineLength/(visibleEnd-visibleStart))
		if screenX >= lineStartX && screenX <= lineEndX {
			// Draw point (small square)
			pointSize := 4.0
			ebitenutil.DrawRect(screen, screenX-pointSize/2, float64(lineY)-pointSize/2, pointSize, pointSize, color.White)
			
			// Draw number (only show every few points to avoid clutter)
			pointIndex := int((x - startPoint) / layerPeriod)
			if pointIndex%5 == 0 || math.Abs(x-float64(layerCenter)) < layerPeriod {
				numText := big.NewInt(int64(x)).String()
				numBounds := text.BoundString(basicfont.Face7x13, numText)
				text.Draw(screen, numText, basicfont.Face7x13, int(screenX)-numBounds.Dx()/2, lineY+20, color.White)
			}
		}
	}
	
	// Draw current index marker
	currentX := lineStartX + (float64(currentInt)-visibleStart)*(lineLength/(visibleEnd-visibleStart))
	if currentX >= lineStartX && currentX <= lineEndX {
		ebitenutil.DrawRect(screen, currentX-2, float64(lineY-10), 4, 20, color.RGBA{255, 255, 0, 255})
	}
	
	// Draw cosine waves
	waveAmplitude := 30.0 // Height of waves
	waveSpacing := 40.0    // Vertical spacing between waves
	
	// Position waves based on their wave number (k value) to maintain consistent
	// vertical positioning across all layers, matching the base layer
	for _, wave := range waves {
		// Use wave number (k value) for positioning instead of array index
		// This ensures wave 1 is always at the same height, wave 2 at the same height, etc.
		waveY := float64(lineY) - float64(wave.WaveNum)*waveSpacing
		
		// Draw cosine wave
		points := []struct{ x, y float64 }{}
		// Use finer step size to ensure smooth waves, especially for larger periods
		stepSize := 0.5
		if wave.Period > 100 {
			stepSize = wave.Period / 200 // Ensure at least 200 points per period
		}
		for x := visibleStart; x <= visibleEnd; x += stepSize {
			// Calculate y position based on cosine
			// Cosine centered at wave.Center with period wave.Period
			// Negate cosine so peaks are at the top (valleys at bottom)
			phase := 2.0 * math.Pi * (x - float64(wave.Center)) / wave.Period
			y := waveY - math.Cos(phase)*waveAmplitude
			
			screenX := lineStartX + (x-visibleStart)*(lineLength/(visibleEnd-visibleStart))
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
			screenX := lineStartX + (x-visibleStart)*(lineLength/(visibleEnd-visibleStart))
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
	controlsText := "LEFT/RIGHT: move by 2 | Shift+LEFT/RIGHT: move by wave period | UP: create new layer | DOWN: go back | C: cycle coefficient | R: reset to -1"
	controlsBounds := text.BoundString(basicfont.Face7x13, controlsText)
	controlsX := (screenWidth - controlsBounds.Dx()) / 2
	controlsY := screenHeight - 15
	text.Draw(screen, controlsText, basicfont.Face7x13, controlsX, controlsY, color.Gray{Y: 150})
}

// WasEscConsumed returns whether ESC was consumed by a dialog this frame
func (w *WavesChapter) WasEscConsumed() bool {
	return w.escConsumed
}
