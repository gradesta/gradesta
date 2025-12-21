package main

import (
	"image/color"
	"math"

	"github.com/hajimehoshi/ebiten/v2"
	"github.com/hajimehoshi/ebiten/v2/ebitenutil"
	"github.com/hajimehoshi/ebiten/v2/inpututil"
	"github.com/hajimehoshi/ebiten/v2/text"
	"golang.org/x/image/font/basicfont"
)

const (
	pointSize      = 8
	pointSpacing   = 50.0  // Distance between points along the line
	pointsToRender = 3000    // Number of points to render around the selected point
)

// Point represents a point on the line
type Point struct {
	X, Y float64
	Index int
}

// StairsChapter implements the Stairs chapter
type StairsChapter struct {
	selectedIndex    int
	cameraX          float64 // Camera position in world units
	cameraY          float64 // Camera position in world units
	zoom             float64 // Zoom factor (world units per pixel)
	originX          float64 // World origin X position in screen coordinates (pixels)
	originY          float64 // World origin Y position in screen coordinates (pixels)
	globalCoeficient float64 // Coefficient for the line equation (y = globalCoeficient*x + 1)
	
	// Cached staircase calculation
	cachedSteps      []StairStep
	cachedIsUpwards  bool
	cachedHitLimit   bool
	cachedForIndex   int
	cachedForCoef    float64
}

// NewStairsChapter creates a new Stairs chapter
func NewStairsChapter() *StairsChapter {
	originX := float64(screenWidth) / 2
	originY := float64(screenHeight) / 2 // World origin (y=0) position in screen coordinates
	
	return &StairsChapter{
		selectedIndex:    1, // Start at an odd index
		cameraX:              0, // Camera at world origin
		cameraY:              0, // Camera at world origin
		zoom:                 1.0 / pointSpacing, // 1 pixel = 1/pointSpacing world units
		originX:               originX,
		originY:               originY,
		globalCoeficient:     3.0, // y = 3x + 1
	}
}

// worldToScreen converts world coordinates (math space) to screen coordinates (pixels)
func (s *StairsChapter) worldToScreen(worldX, worldY float64) (screenX, screenY float64) {
	// Apply camera offset (camera is in world units)
	offsetX := worldX - s.cameraX
	offsetY := worldY - s.cameraY
	
	// Convert world units to pixels using zoom (zoom = world units per pixel)
	pixelX := offsetX / s.zoom
	pixelY := offsetY / s.zoom
	
	// Transform to screen coordinates (origin is center, y is inverted)
	screenX = s.originX + pixelX
	screenY = s.originY - pixelY // Invert Y for screen coords
	
	return screenX, screenY
}

// screenToWorld converts screen coordinates (pixels) to world coordinates (math space)
func (s *StairsChapter) screenToWorld(screenX, screenY float64) (worldX, worldY float64) {
	// Convert screen coordinates to pixels relative to origin
	pixelX := screenX - s.originX
	pixelY := s.originY - screenY // Invert Y
	
	// Convert pixels to world units using zoom
	worldOffsetX := pixelX * s.zoom
	worldOffsetY := pixelY * s.zoom
	
	// Apply camera offset (camera is in world units)
	worldX = worldOffsetX + s.cameraX
	worldY = worldOffsetY + s.cameraY
	
	return worldX, worldY
}

// getPointForIndex calculates the point for a given index procedurally
// Returns point in world coordinates (math space)
func (s *StairsChapter) getPointForIndex(index int) (worldX, worldY float64) {
	// Calculate world x coordinate in world units (1 unit per index)
	worldX = float64(index)
	
	// Calculate world y coordinate: y = globalCoeficient*x + 1 (in world units)
	worldY = s.globalCoeficient*worldX + 1.0
	
	return worldX, worldY
}

// getPointForIndexScreen returns the point in screen coordinates for drawing
func (s *StairsChapter) getPointForIndexScreen(index int) Point {
	worldX, worldY := s.getPointForIndex(index)
	screenX, screenY := s.worldToScreen(worldX, worldY)
	
	return Point{
		X:     screenX,
		Y:     screenY,
		Index: index,
	}
}

func (s *StairsChapter) Update() error {
	needsRecalc := false
	
	// Handle 'c' key to cycle through global coefficient values (odd numbers 1-21)
	if inpututil.IsKeyJustPressed(ebiten.KeyC) {
		s.cycleGlobalCoeficient()
		needsRecalc = true
		s.updateCamera()
	}
	
	// Handle Shift+Right Arrow to jump to end of stairs (top or bottom)
	if ebiten.IsKeyPressed(ebiten.KeyShiftLeft) || ebiten.IsKeyPressed(ebiten.KeyShiftRight) {
		if inpututil.IsKeyJustPressed(ebiten.KeyArrowRight) {
			endIndex := s.getEndOfStairs(s.selectedIndex)
			if endIndex != s.selectedIndex {
				s.selectedIndex = endIndex
				needsRecalc = true
				s.updateCamera()
			}
			return nil
		}
	}
	
	// Handle arrow key input - can go infinitely in either direction
	// Only allow odd indexes, so skip by 2
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowRight) || inpututil.IsKeyJustPressed(ebiten.KeyD) {
		s.selectedIndex += 2
		// Ensure we stay on odd index
		if s.selectedIndex%2 == 0 {
			s.selectedIndex++
		}
		needsRecalc = true
		s.updateCamera()
	}
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowLeft) || inpututil.IsKeyJustPressed(ebiten.KeyA) {
		s.selectedIndex -= 2
		// Ensure we stay on odd index
		if s.selectedIndex%2 == 0 {
			s.selectedIndex--
		}
		needsRecalc = true
		s.updateCamera()
	}
	
	// Invalidate cache if needed
	if needsRecalc {
		s.cachedForIndex = -1 // Invalidate cache
	}
	
	return nil
}

func (s *StairsChapter) updateCamera() {
	// Calculate the entire staircase (use cache)
	steps, _, _ := s.getCachedStaircase()
	
	if len(steps) > 0 {
		// Calculate bounding box of entire staircase in world coordinates
		minWorldX := math.Inf(1)
		maxWorldX := math.Inf(-1)
		minWorldY := math.Inf(1)
		maxWorldY := math.Inf(-1)
		
		// Start with the selected point
		startWorldX, startWorldY := s.getPointForIndex(s.selectedIndex)
		minWorldX = math.Min(minWorldX, startWorldX)
		maxWorldX = math.Max(maxWorldX, startWorldX)
		minWorldY = math.Min(minWorldY, startWorldY)
		maxWorldY = math.Max(maxWorldY, startWorldY)
		
		// Process each step in the staircase
		currentIndex := float64(s.selectedIndex)
		for _, step := range steps {
			// Get start point for this step
			stepStartX, stepStartY := s.getPointForIndex(int(currentIndex))
			
			// Calculate 2^k for this step
			powerOf2 := math.Pow(2.0, float64(step.K))
			
			// Find intersection of horizontal line from start point with 2^k line
			intersectionWorldX := stepStartY / powerOf2
			
			// Find point on globalCoeficient*x+1 line at same x as intersection
			originalWorldY := s.globalCoeficient*intersectionWorldX + 1.0
			
			// Get destination point
			destWorldX := step.DestinationIndex
			destWorldY := s.globalCoeficient*destWorldX + 1.0
			
			// Update bounding box with all triangle vertices
			minWorldX = math.Min(minWorldX, math.Min(stepStartX, math.Min(intersectionWorldX, destWorldX)))
			maxWorldX = math.Max(maxWorldX, math.Max(stepStartX, math.Max(intersectionWorldX, destWorldX)))
			minWorldY = math.Min(minWorldY, math.Min(stepStartY, math.Min(originalWorldY, destWorldY)))
			maxWorldY = math.Max(maxWorldY, math.Max(stepStartY, math.Max(originalWorldY, destWorldY)))
			
			// Move to destination for next step
			currentIndex = step.DestinationIndex
		}
		
		// Add padding in world units
		paddingWorld := 2.0
		minWorldX -= paddingWorld
		maxWorldX += paddingWorld
		minWorldY -= paddingWorld
		maxWorldY += paddingWorld
		
		// Calculate center and size in world units
		centerWorldX := (minWorldX + maxWorldX) / 2
		centerWorldY := (minWorldY + maxWorldY) / 2
		widthWorld := maxWorldX - minWorldX
		heightWorld := maxWorldY - minWorldY
		
		// Calculate required zoom to fit the entire staircase
		// Available screen space (with some margin)
		availableWidth := float64(screenWidth) * 0.9
		availableHeight := float64(screenHeight) * 0.9
		
		// Calculate required zoom (world units per pixel)
		zoomX := widthWorld / availableWidth
		zoomY := heightWorld / availableHeight
		requiredZoom := math.Max(zoomX, zoomY)
		
		// Apply the new zoom
		s.zoom = requiredZoom
		
		// Center camera on the staircase's center
		s.cameraX = centerWorldX
		s.cameraY = centerWorldY
	} else {
		// No staircase, just center on selected point
		selectedWorldX, selectedWorldY := s.getPointForIndex(s.selectedIndex)
		s.cameraX = selectedWorldX
		s.cameraY = selectedWorldY
		// Keep current zoom or reset to default
		if s.zoom == 0 {
			s.zoom = 1.0 / pointSpacing
		}
	}
}

func (s *StairsChapter) Draw(screen *ebiten.Image) {
	// Draw axes at point 0
	s.drawAxes(screen)
	
	// Draw the line with slope globalCoeficient*x+1 procedurally
	s.drawLine(screen, s.globalCoeficient, 1.0, color.Gray{Y: 100})
	
	// Draw the triangle for the selected point
	s.drawTriangle(screen)
	
	// Draw visible points procedurally
	s.drawPoints(screen)
	
	// Draw destination point if it exists
	s.drawDestinationPoint(screen)
	
	// Draw selected point index and y value at the bottom
	s.drawIndex(screen)
}

func (s *StairsChapter) drawAxes(screen *ebiten.Image) {
	// Draw axes at world origin (0, 0) in math space
	// Project to screen coordinates
	originScreenX, originScreenY := s.worldToScreen(0, 0)
	
	// Draw vertical line (y-axis) - extends up and down from origin
	// Use world units for axis length, then project
	axisLength := 100.0 // 100 world units
	axisColor := color.RGBA{100, 100, 255, 255} // Blue for axes
	
	// Vertical line: from (0, -axisLength) to (0, axisLength) in world coords
	_, topY := s.worldToScreen(0, axisLength)
	_, bottomY := s.worldToScreen(0, -axisLength)
	
	// Draw vertical line
	for y := math.Min(topY, bottomY); y <= math.Max(topY, bottomY); y++ {
		if y >= 0 && y < float64(screenHeight) {
			ebitenutil.DrawRect(screen, originScreenX-0.5, y-0.5, 1, 1, axisColor)
		}
	}
	
	// Horizontal line (x-axis): from (-axisLength, 0) to (axisLength, 0) in world coords
	leftX, _ := s.worldToScreen(-axisLength, 0)
	rightX, _ := s.worldToScreen(axisLength, 0)
	
	// Draw horizontal line
	for x := math.Min(leftX, rightX); x <= math.Max(leftX, rightX); x++ {
		if x >= 0 && x < float64(screenWidth) {
			ebitenutil.DrawRect(screen, x-0.5, originScreenY-0.5, 1, 1, axisColor)
		}
	}
}

func (s *StairsChapter) drawLine(screen *ebiten.Image, riseOverRun, constant float64, lineColor color.Color) {
	// Draw a line procedurally in math space (world coordinates)
	// Equation: y = riseOverRun*x + constant (in world units)
	
	// Calculate visible range in world coordinates
	// Convert screen bounds to world coordinates
	minWorldX, _ := s.screenToWorld(-100, 0)
	maxWorldX, _ := s.screenToWorld(float64(screenWidth)+100, 0)
	
	// Calculate range size
	rangeSize := math.Abs(maxWorldX - minWorldX)
	if rangeSize == 0 {
		return // No range to draw
	}
	
	// Limit the number of iterations to prevent hanging
	maxIterations := 2000
	
	// Calculate step size to ensure we don't exceed max iterations
	// But also ensure we have enough points for a visible line
	minIterations := 100 // Minimum points to draw a visible line
	stepSize := rangeSize / float64(maxIterations)
	
	// If step size would result in too few points, use a smaller step size
	if rangeSize/stepSize < float64(minIterations) {
		stepSize = rangeSize / float64(minIterations)
	}
	
	// Clamp step size to reasonable bounds
	if stepSize < 0.01 {
		stepSize = 0.01 // Minimum step size for smooth lines when zoomed in
	}
	if stepSize > 10.0 {
		stepSize = 10.0 // Maximum step size to prevent gaps
	}
	
	iterations := 0
	
	// Draw the line with adaptive step size
	for worldX := minWorldX; worldX <= maxWorldX && iterations < maxIterations; worldX += stepSize {
		iterations++
		
		// Calculate world y: y = riseOverRun*x + constant (in world units)
		worldY := riseOverRun*worldX + constant
		
		// Project to screen coordinates
		screenX, screenY := s.worldToScreen(worldX, worldY)
		
		// Only draw if on screen
		if screenX >= 0 && screenX < float64(screenWidth) && 
		   screenY >= 0 && screenY < float64(screenHeight) {
			ebitenutil.DrawRect(screen, screenX-0.5, screenY-0.5, 1, 1, lineColor)
		}
	}
}

func (s *StairsChapter) drawPoints(screen *ebiten.Image) {
	// Draw points around the selected index, but only odd indexes
	startIndex := s.selectedIndex - pointsToRender
	endIndex := s.selectedIndex + pointsToRender
	
		for i := startIndex; i <= endIndex; i++ {
		// Only draw odd indexes
		if i%2 != 0 {
			point := s.getPointForIndexScreen(i)
			isSelected := (i == s.selectedIndex)
			s.drawPoint(screen, point, isSelected)
		}
	}
}

func (s *StairsChapter) drawPoint(screen *ebiten.Image, point Point, selected bool) {
	// Point is already in screen coordinates (from getPointForIndexScreen)
	x := point.X
	y := point.Y
	
	// Only draw if on screen (with some padding)
	if x < -pointSize || x > float64(screenWidth)+pointSize ||
		y < -pointSize || y > float64(screenHeight)+pointSize {
		return
	}
	
	// Choose color based on selection
	clr := color.RGBA{150, 150, 150, 255}
	if selected {
		clr = color.RGBA{255, 100, 100, 255}
	}
	
	// Draw point as a circle (approximated with a filled rectangle)
	ebitenutil.DrawRect(screen, x-pointSize/2, y-pointSize/2, pointSize, pointSize, clr)
	
	// Draw a border for selected point
	if selected {
		ebitenutil.DrawRect(screen, x-pointSize/2-1, y-pointSize/2-1, pointSize+2, 1, color.White)
		ebitenutil.DrawRect(screen, x-pointSize/2-1, y-pointSize/2-1, 1, pointSize+2, color.White)
		ebitenutil.DrawRect(screen, x+pointSize/2, y-pointSize/2-1, 1, pointSize+2, color.White)
		ebitenutil.DrawRect(screen, x-pointSize/2-1, y+pointSize/2, pointSize+2, 1, color.White)
	}
}

// calculateStaircase calculates all steps from startIndex to the end of the stairs
// Returns a slice of StairStep, where each step's destination is different from its start
// Also returns true if the staircase goes upwards, false if downwards
// Third return value is true if the 50-step limit was hit
// Maximum of 50 steps to prevent infinite loops
func (s *StairsChapter) calculateStaircase(startIndex int) ([]StairStep, bool, bool) {
	maxSteps := 50
	return CalculateStaircase(startIndex, s.globalCoeficient, maxSteps, true) // Stop on direction change
}

// getCachedStaircase returns the cached staircase or calculates it if cache is invalid
func (s *StairsChapter) getCachedStaircase() ([]StairStep, bool, bool) {
	// Check if cache is valid
	if s.cachedForIndex == s.selectedIndex && s.cachedForCoef == s.globalCoeficient {
		return s.cachedSteps, s.cachedIsUpwards, s.cachedHitLimit
	}
	
	// Cache is invalid, recalculate
	steps, isUpwards, hitLimit := s.calculateStaircase(s.selectedIndex)
	
	// Update cache
	s.cachedSteps = steps
	s.cachedIsUpwards = isUpwards
	s.cachedHitLimit = hitLimit
	s.cachedForIndex = s.selectedIndex
	s.cachedForCoef = s.globalCoeficient
	
	return steps, isUpwards, hitLimit
}

// getEndOfStairs returns the end index (top or bottom) of the staircase starting from startIndex
func (s *StairsChapter) getEndOfStairs(startIndex int) int {
	// For this function, we need to calculate for a specific index, not the cached one
	steps, _, _ := s.calculateStaircase(startIndex)
	if len(steps) == 0 {
		return startIndex
	}
	// Return the last destination as an integer (round to nearest odd)
	endIndex := int(steps[len(steps)-1].DestinationIndex)
	// Ensure it's odd
	if endIndex%2 == 0 {
		endIndex++
	}
	return endIndex
}

// cycleGlobalCoeficient cycles through odd numbers from 1 to 21
func (s *StairsChapter) cycleGlobalCoeficient() {
	// Odd numbers from 1 to 21: 1, 3, 5, 7, 9, 11, 13, 15, 17, 19, 21
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

func (s *StairsChapter) drawTriangle(screen *ebiten.Image) {
	// Calculate the entire staircase (use cache)
	steps, _, _ := s.getCachedStaircase()
	
	if len(steps) == 0 {
		return // No staircase to draw
	}
	
		// Draw each step in the staircase
		currentIndex := float64(s.selectedIndex)
		for _, step := range steps {
			// Get start point for this step
			_, stepStartY := s.getPointForIndex(int(currentIndex))
		
		// Calculate 2^k for this step
		powerOf2 := math.Pow(2.0, float64(step.K))
		
		// The new line has slope 2^k and passes through origin (0,0) in world coordinates
		// So in world coordinates: y = 2^k * x (riseOverRun = 2^k, constant = 0)
		
		// Draw the line with slope 2^k using the drawLine function
		lineColor := color.RGBA{100, 255, 100, 255} // Green for the new line
		s.drawLine(screen, powerOf2, 0.0, lineColor)
		
		// Find intersection of horizontal line from start point with the new line
		// In world units:
		// Horizontal line: y = stepStartY
		// New line: y = 2^k * x
		// So: stepStartY = 2^k * x
		// x = stepStartY / 2^k
		intersectionWorldX := stepStartY / powerOf2
		intersectionWorldY := stepStartY
		
		// Project intersection to screen coordinates
		intersectionScreenX, intersectionScreenY := s.worldToScreen(intersectionWorldX, intersectionWorldY)
		intersectionPoint := Point{
			X: intersectionScreenX,
			Y: intersectionScreenY,
		}
		
		// Get start point in screen coordinates for drawing
		startPoint := s.getPointForIndexScreen(int(currentIndex))
		
		// Draw horizontal line from start point to intersection
		horizontalColor := color.RGBA{255, 255, 100, 255} // Yellow for horizontal
		s.drawLineSegment(screen, startPoint, intersectionPoint, horizontalColor)
		
		// Find the point on the original line (y = globalCoeficient*x + 1 in world units) at the same x as intersection
		// In world units: y = globalCoeficient * intersectionWorldX + 1
		originalWorldY := s.globalCoeficient*intersectionWorldX + 1.0
		
		// Project to screen coordinates
		originalScreenX, originalScreenY := s.worldToScreen(intersectionWorldX, originalWorldY)
		originalLinePoint := Point{
			X: originalScreenX,
			Y: originalScreenY,
		}
		
		// Draw vertical line from intersection to original line
		verticalColor := color.RGBA{255, 100, 255, 255} // Magenta for vertical
		s.drawLineSegment(screen, intersectionPoint, originalLinePoint, verticalColor)
		
		// Move to destination for next step
		currentIndex = step.DestinationIndex
	}
}

func (s *StairsChapter) drawLineSegment(screen *ebiten.Image, p1, p2 Point, clr color.Color) {
	// Points are already in screen coordinates (from projection)
	x1 := p1.X
	y1 := p1.Y
	x2 := p2.X
	y2 := p2.Y
	
	// Draw line using multiple small rectangles
	dx := x2 - x1
	dy := y2 - y1
	length := math.Sqrt(dx*dx + dy*dy)
	steps := int(length * 2) // Higher resolution
	
	if steps > 0 {
		stepX := dx / float64(steps)
		stepY := dy / float64(steps)
		
		for j := 0; j <= steps; j++ {
			x := x1 + float64(j)*stepX
			y := y1 + float64(j)*stepY
			if x >= 0 && x < float64(screenWidth) && y >= 0 && y < float64(screenHeight) {
				ebitenutil.DrawRect(screen, x-0.5, y-0.5, 1, 1, clr)
			}
		}
	}
}

func (s *StairsChapter) drawDestinationPoint(screen *ebiten.Image) {
	// Draw all destination points in the staircase (use cache)
	steps, _, _ := s.getCachedStaircase()
	
	for _, step := range steps {
		// Calculate the destination point in world coordinates
		// The destination is on the globalCoeficient*x+1 line at x = destinationIndex
		destinationWorldX := step.DestinationIndex
		destinationWorldY := s.globalCoeficient*destinationWorldX + 1.0
		
		// Project to screen coordinates
		destScreenX, destScreenY := s.worldToScreen(destinationWorldX, destinationWorldY)
		
		// Only draw if on screen
		if destScreenX >= -pointSize*2 && destScreenX < float64(screenWidth)+pointSize*2 &&
			destScreenY >= -pointSize*2 && destScreenY < float64(screenHeight)+pointSize*2 {
			
			// Draw a larger, highlighted point
			destColor := color.RGBA{100, 255, 255, 255} // Cyan for destination
			destSize := float64(pointSize + 4)
			ebitenutil.DrawRect(screen, destScreenX-destSize/2, destScreenY-destSize/2, destSize, destSize, destColor)
			
			// Draw a bright border
			ebitenutil.DrawRect(screen, destScreenX-destSize/2-2, destScreenY-destSize/2-2, destSize+4, 2, color.White)
			ebitenutil.DrawRect(screen, destScreenX-destSize/2-2, destScreenY-destSize/2-2, 2, destSize+4, color.White)
			ebitenutil.DrawRect(screen, destScreenX+destSize/2, destScreenY-destSize/2-2, 2, destSize+4, color.White)
			ebitenutil.DrawRect(screen, destScreenX-destSize/2-2, destScreenY+destSize/2, destSize+4, 2, color.White)
		}
	}
}

func (s *StairsChapter) drawIndex(screen *ebiten.Image) {
	// Calculate staircase steps (use cache)
	steps, isUpwards, hitLimit := s.getCachedStaircase()
	
	// Use the utility function to draw the table
	data := CollatzTableData{
		Steps:      steps,
		StartIndex: s.selectedIndex,
		Coefficient: s.globalCoeficient,
		IsUpwards:  isUpwards,
		HitLimit:   hitLimit,
		StopOnDirectionChange: true, // Stop when direction changes
	}
	DrawCollatzTable(screen, data)
	
	// Draw additional info at the very bottom
	stepsCount := len(steps)
	var infoText string
	if stepsCount > 0 {
		if isUpwards {
			infoText = "Arrow/A/D: navigate | Shift+Right: jump to top | C: change coefficient"
		} else {
			infoText = "Arrow/A/D: navigate | Shift+Right: jump to bottom | C: change coefficient"
		}
	} else {
		infoText = "Arrow/A/D: navigate | Shift+Right: jump to end | C: change coefficient"
	}
	infoBounds := text.BoundString(basicfont.Face7x13, infoText)
	infoX := (screenWidth - infoBounds.Dx()) / 2
	infoY := screenHeight - 2
	text.Draw(screen, infoText, basicfont.Face7x13, infoX, infoY, color.Gray{Y: 100})
}

