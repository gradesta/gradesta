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
	pointSize      = 8
	pointSpacing   = 50.0  // Distance between points along the line
	pointsToRender = 30    // Number of points to render around the selected point
)

// Point represents a point on the line
type Point struct {
	X, Y float64
	Index int
}

// StairsChapter implements the Stairs chapter
type StairsChapter struct {
	selectedIndex int
	cameraX       float64
	cameraY       float64
	originX       float64 // World origin X position in screen coordinates
	originY       float64 // World origin Y position in screen coordinates (y=0 in world coords)
}

// NewStairsChapter creates a new Stairs chapter
func NewStairsChapter() *StairsChapter {
	originX := 100.0
	originY := 450.0 // World origin (y=0) position in screen coordinates
	
	return &StairsChapter{
		selectedIndex: 1, // Start at an odd index
		cameraX:        0,
		cameraY:        0,
		originX:        originX,
		originY:        originY,
	}
}

// getPointForIndex calculates the point for a given index procedurally
// Uses world coordinates where y increases upward, then converts to screen coordinates
func (s *StairsChapter) getPointForIndex(index int) Point {
	// Calculate world x coordinate in pixels (index * pointSpacing)
	worldX := float64(index) * pointSpacing
	
	// Calculate world y coordinate: y = 3x + 1
	// x is in world units (1 unit per index), so convert index to units, calculate, then convert to pixels
	worldXUnits := float64(index)
	worldYUnits := 3.0*worldXUnits + 1.0
	worldY := worldYUnits * pointSpacing
	
	// Convert world coordinates to screen coordinates
	// Screen x = originX + worldX
	// Screen y = originY - worldY (invert because screen y increases downward)
	screenX := s.originX + worldX
	screenY := s.originY - worldY
	
	return Point{
		X:     screenX,
		Y:     screenY,
		Index: index,
	}
}

func (s *StairsChapter) Update() error {
	// Handle arrow key input - can go infinitely in either direction
	// Only allow odd indexes, so skip by 2
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowRight) || inpututil.IsKeyJustPressed(ebiten.KeyD) {
		s.selectedIndex += 2
		// Ensure we stay on odd index
		if s.selectedIndex%2 == 0 {
			s.selectedIndex++
		}
		s.updateCamera()
	}
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowLeft) || inpututil.IsKeyJustPressed(ebiten.KeyA) {
		s.selectedIndex -= 2
		// Ensure we stay on odd index
		if s.selectedIndex%2 == 0 {
			s.selectedIndex--
		}
		s.updateCamera()
	}
	
	return nil
}

func (s *StairsChapter) updateCamera() {
	// Get the selected point
	selectedPoint := s.getPointForIndex(s.selectedIndex)
	
	// Calculate y value to check if triangle should be shown
	yValue := int(3.0*float64(s.selectedIndex) + 1.0)
	n := findLargestPowerOf2(yValue)
	
	if n > 0 {
		// Calculate triangle bounds to fit it on screen
		powerOf2 := math.Pow(2.0, float64(n))
		selectedWorldY := s.originY - selectedPoint.Y
		intersectionWorldX := selectedWorldY / powerOf2
		originalWorldY := 3.0*intersectionWorldX + 1.0
		
		// Get triangle points in screen coordinates
		intersectionScreenX := s.originX + intersectionWorldX
		intersectionScreenY := s.originY - selectedWorldY
		originalScreenX := s.originX + intersectionWorldX
		originalScreenY := s.originY - originalWorldY
		
		// Find bounding box of triangle
		minX := math.Min(selectedPoint.X, math.Min(intersectionScreenX, originalScreenX))
		maxX := math.Max(selectedPoint.X, math.Max(intersectionScreenX, originalScreenX))
		minY := math.Min(selectedPoint.Y, math.Min(intersectionScreenY, originalScreenY))
		maxY := math.Max(selectedPoint.Y, math.Max(intersectionScreenY, originalScreenY))
		
		// Add padding
		padding := 50.0
		minX -= padding
		maxX += padding
		minY -= padding
		maxY += padding
		
		// Calculate center and size
		centerX := (minX + maxX) / 2
		centerY := (minY + maxY) / 2
		width := maxX - minX
		height := maxY - minY
		
		// Calculate scale needed to fit
		scaleX := float64(screenWidth) / width
		scaleY := float64(screenHeight) / height
		scale := math.Min(scaleX, scaleY)
		
		// If triangle fits, center it; otherwise center on selected point
		if scale < 1.0 {
			// Need to zoom out - center on triangle
			s.cameraX = centerX - screenWidth/2
			s.cameraY = centerY - screenHeight/2
		} else {
			// Triangle fits, center on selected point
			s.cameraX = selectedPoint.X - screenWidth/2
			s.cameraY = selectedPoint.Y - screenHeight/2
		}
	} else {
		// No triangle, just center on selected point
		s.cameraX = selectedPoint.X - screenWidth/2
		s.cameraY = selectedPoint.Y - screenHeight/2
	}
}

func (s *StairsChapter) Draw(screen *ebiten.Image) {
	// Draw axes at point 0
	s.drawAxes(screen)
	
	// Draw the line with slope 3x+1 procedurally
	s.drawLine(screen, 3.0, 1.0, color.Gray{Y: 100})
	
	// Draw the triangle for the selected point
	s.drawTriangle(screen)
	
	// Draw visible points procedurally
	s.drawPoints(screen)
	
	// Draw selected point index and y value at the bottom
	s.drawIndex(screen)
}

func (s *StairsChapter) drawAxes(screen *ebiten.Image) {
	// Draw axes at world origin (0, 0)
	// Transform world origin to screen coordinates
	originX := s.originX - s.cameraX
	originY := s.originY - s.cameraY
	
	// Draw vertical line (y-axis) - extends up and down from origin
	axisLength := 200.0
	axisColor := color.RGBA{100, 100, 255, 255} // Blue for axes
	
	// Vertical line
	for y := originY - axisLength; y <= originY + axisLength; y++ {
		if y >= 0 && y < screenHeight {
			ebitenutil.DrawRect(screen, originX-0.5, y-0.5, 1, 1, axisColor)
		}
	}
	
	// Horizontal line (x-axis) - extends left and right from origin
	for x := originX - axisLength; x <= originX + axisLength; x++ {
		if x >= 0 && x < screenWidth {
			ebitenutil.DrawRect(screen, x-0.5, originY-0.5, 1, 1, axisColor)
		}
	}
}

func (s *StairsChapter) drawLine(screen *ebiten.Image, riseOverRun, constant float64, lineColor color.Color) {
	// Draw a line procedurally using world coordinates with equation y = riseOverRun*x + constant
	// Use the same coordinate system as getPointForIndex
	// Calculate visible range in world coordinates based on what's on screen
	
	// Screen coordinates visible range
	minScreenX := -100.0
	maxScreenX := float64(screenWidth) + 100.0
	
	// Convert to world x coordinates
	// Points use: screenX = originX + worldX (without camera in stored coords)
	// But when drawing, we apply camera: actualScreenX = storedX - cameraX
	// So: actualScreenX = originX + worldX - cameraX
	// Therefore: worldX = actualScreenX - originX + cameraX
	minWorldX := (minScreenX - s.originX + s.cameraX)
	maxWorldX := (maxScreenX - s.originX + s.cameraX)
	
	// Draw the line directly using the equation y = riseOverRun*x + constant
	// Note: worldX is in pixels, but the equation expects world units
	// We need to normalize: 1 world unit = pointSpacing pixels
	stepSize := 0.5
	for worldX := minWorldX; worldX <= maxWorldX; worldX += stepSize {
		// Convert worldX from pixels to world units
		worldXUnits := worldX / pointSpacing
		// Calculate world y in world units: y = riseOverRun*x + constant
		// At world x=0, this gives world y=constant
		worldYUnits := riseOverRun*worldXUnits + constant
		// Convert back to pixels
		worldY := worldYUnits * pointSpacing
		
		// Convert to screen coordinates using the same formula as getPointForIndex
		// Then apply camera offset when drawing
		storedScreenX := s.originX + worldX
		storedScreenY := s.originY - worldY
		
		// Apply camera transform for actual screen position
		actualScreenX := storedScreenX - s.cameraX
		actualScreenY := storedScreenY - s.cameraY
		
		// Only draw if on screen
		if actualScreenX >= 0 && actualScreenX < float64(screenWidth) && 
		   actualScreenY >= 0 && actualScreenY < float64(screenHeight) {
			ebitenutil.DrawRect(screen, actualScreenX-0.5, actualScreenY-0.5, 1, 1, lineColor)
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
			point := s.getPointForIndex(i)
			isSelected := (i == s.selectedIndex)
			s.drawPoint(screen, point, isSelected)
		}
	}
}

func (s *StairsChapter) drawPoint(screen *ebiten.Image, point Point, selected bool) {
	// Transform point to screen coordinates
	x := point.X - s.cameraX
	y := point.Y - s.cameraY
	
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

// findLargestPowerOf2 finds the largest n such that y is divisible by 2^n
func findLargestPowerOf2(y int) int {
	if y == 0 {
		return 0
	}
	n := 0
	for y%2 == 0 {
		y /= 2
		n++
	}
	return n
}

func (s *StairsChapter) drawTriangle(screen *ebiten.Image) {
	// Get the selected point
	selectedPoint := s.getPointForIndex(s.selectedIndex)
	
	// Calculate y value in world coordinates
	yValue := int(3.0*float64(s.selectedIndex) + 1.0)
	
	// Find the largest n where y is divisible by 2^n
	n := findLargestPowerOf2(yValue)
	if n == 0 {
		return // No triangle if y is odd
	}
	
	// Calculate 2^n
	powerOf2 := math.Pow(2.0, float64(n))
	
	// The new line has slope 2^n and passes through origin (0,0) in world coordinates
	// So in world coordinates: y = 2^n * x (riseOverRun = 2^n, constant = 0)
	
	// Draw the line with slope 2^n using the drawLine function
	lineColor := color.RGBA{100, 255, 100, 255} // Green for the new line
	s.drawLine(screen, powerOf2, 0.0, lineColor)
	
	// Convert selected point to world coordinates (in world units, not pixels)
	// selectedPoint is in screen coordinates, convert to world units
	selectedWorldYPixels := s.originY - selectedPoint.Y // Invert for world coords
	selectedWorldYUnits := selectedWorldYPixels / pointSpacing
	
	// Find intersection of horizontal line from selected point with the new line
	// In world units:
	// Horizontal line: y = selectedWorldYUnits
	// New line: y = 2^n * x
	// So: selectedWorldYUnits = 2^n * x
	// x = selectedWorldYUnits / 2^n
	intersectionWorldXUnits := selectedWorldYUnits / powerOf2
	intersectionWorldYUnits := selectedWorldYUnits
	
	// Convert intersection from world units to pixels, then to screen coordinates
	intersectionWorldXPixels := intersectionWorldXUnits * pointSpacing
	intersectionWorldYPixels := intersectionWorldYUnits * pointSpacing
	intersectionScreenX := s.originX + intersectionWorldXPixels
	intersectionScreenY := s.originY - intersectionWorldYPixels
	intersectionPoint := Point{
		X: intersectionScreenX,
		Y: intersectionScreenY,
	}
	
	// Draw horizontal line from selected point to intersection
	horizontalColor := color.RGBA{255, 255, 100, 255} // Yellow for horizontal
	s.drawLineSegment(screen, selectedPoint, intersectionPoint, horizontalColor)
	
	// Find the point on the original line (y = 3x + 1 in world units) at the same x as intersection
	// In world units: y = 3 * intersectionWorldXUnits + 1
	originalWorldYUnits := 3.0*intersectionWorldXUnits + 1.0
	
	// Convert to pixels, then to screen coordinates
	originalWorldYPixels := originalWorldYUnits * pointSpacing
	originalScreenX := s.originX + intersectionWorldXPixels
	originalScreenY := s.originY - originalWorldYPixels
	originalLinePoint := Point{
		X: originalScreenX,
		Y: originalScreenY,
	}
	
	// Draw vertical line from intersection to original line
	verticalColor := color.RGBA{255, 100, 255, 255} // Magenta for vertical
	s.drawLineSegment(screen, intersectionPoint, originalLinePoint, verticalColor)
}

func (s *StairsChapter) drawLineSegment(screen *ebiten.Image, p1, p2 Point, clr color.Color) {
	// Transform points to screen coordinates
	x1 := p1.X - s.cameraX
	y1 := p1.Y - s.cameraY
	x2 := p2.X - s.cameraX
	y2 := p2.Y - s.cameraY
	
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

func (s *StairsChapter) drawIndex(screen *ebiten.Image) {
	// Calculate the mathematical y value (3x + 1) where x is the index
	yValue := 3.0*float64(s.selectedIndex) + 1.0
	
	// Format the display text
	indexText := "Index: " + strconv.Itoa(s.selectedIndex)
	yText := "y = " + strconv.FormatFloat(yValue, 'f', 1, 64)
	
	// Draw index
	indexBounds := text.BoundString(basicfont.Face7x13, indexText)
	indexX := (screenWidth - indexBounds.Dx()) / 2
	indexY := screenHeight - 45
	text.Draw(screen, indexText, basicfont.Face7x13, indexX, indexY, color.White)
	
	// Draw y value
	yBounds := text.BoundString(basicfont.Face7x13, yText)
	yX := (screenWidth - yBounds.Dx()) / 2
	yY := screenHeight - 30
	text.Draw(screen, yText, basicfont.Face7x13, yX, yY, color.White)
	
	// Draw additional info
	infoText := "Use Arrow Keys or A/D to navigate"
	infoBounds := text.BoundString(basicfont.Face7x13, infoText)
	infoX := (screenWidth - infoBounds.Dx()) / 2
	infoY := screenHeight - 15
	text.Draw(screen, infoText, basicfont.Face7x13, infoX, infoY, color.Gray{Y: 100})
}

