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
	pointsToRender = 3000    // Number of points to render around the selected point
)

// Point represents a point on the line
type Point struct {
	X, Y float64
	Index int
}

// StairsChapter implements the Stairs chapter
type StairsChapter struct {
	selectedIndex int
	cameraX       float64 // Camera position in world units
	cameraY       float64 // Camera position in world units
	zoom          float64 // Zoom factor (world units per pixel)
	originX       float64 // World origin X position in screen coordinates (pixels)
	originY       float64 // World origin Y position in screen coordinates (pixels)
}

// NewStairsChapter creates a new Stairs chapter
func NewStairsChapter() *StairsChapter {
	originX := float64(screenWidth) / 2
	originY := float64(screenHeight) / 2 // World origin (y=0) position in screen coordinates
	
	return &StairsChapter{
		selectedIndex: 1, // Start at an odd index
		cameraX:        0, // Camera at world origin
		cameraY:        0, // Camera at world origin
		zoom:           1.0 / pointSpacing, // 1 pixel = 1/pointSpacing world units
		originX:        originX,
		originY:        originY,
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
	
	// Calculate world y coordinate: y = 3x + 1 (in world units)
	worldY = 3.0*worldX + 1.0
	
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
	// Get the selected point in world coordinates
	selectedWorldX, selectedWorldY := s.getPointForIndex(s.selectedIndex)
	
	// Calculate y value to check if triangle should be shown
	yValue := int(3.0*float64(s.selectedIndex) + 1.0)
	k := findLargestPowerOf2(yValue)
	
	if k > 0 {
		// Calculate triangle bounds in world coordinates (math space)
		powerOf2 := math.Pow(2.0, float64(k))
		
		// Find intersection of horizontal line from selected point with 2^k line
		// Horizontal line: y = selectedWorldY
		// 2^k line: y = 2^k * x
		// So: selectedWorldY = 2^k * x
		intersectionWorldX := selectedWorldY / powerOf2
		intersectionWorldY := selectedWorldY
		
		// Find point on 3x+1 line at same x as intersection
		originalWorldY := 3.0*intersectionWorldX + 1.0
		
		// Triangle vertices in world coordinates
		// 1. Selected point: (selectedWorldX, selectedWorldY)
		// 2. Intersection: (intersectionWorldX, intersectionWorldY)
		// 3. Original line point: (intersectionWorldX, originalWorldY)
		
		// Find bounding box in world coordinates
		minWorldX := math.Min(selectedWorldX, intersectionWorldX)
		maxWorldX := math.Max(selectedWorldX, intersectionWorldX)
		minWorldY := math.Min(selectedWorldY, math.Min(intersectionWorldY, originalWorldY))
		maxWorldY := math.Max(selectedWorldY, math.Max(intersectionWorldY, originalWorldY))
		
		// Add padding in world units
		padding := 2.0 // 2 world units of padding
		minWorldX -= padding
		maxWorldX += padding
		minWorldY -= padding
		maxWorldY += padding
		
		// Calculate center in world coordinates
		centerWorldX := (minWorldX + maxWorldX) / 2
		centerWorldY := (minWorldY + maxWorldY) / 2
		
		// Calculate size in world units
		widthWorld := maxWorldX - minWorldX
		heightWorld := maxWorldY - minWorldY
		
		// Calculate zoom needed to fit triangle on screen
		// Available screen space (with some margin)
		availableWidth := float64(screenWidth) * 0.9
		availableHeight := float64(screenHeight) * 0.9
		
		// Calculate required zoom (world units per pixel)
		zoomX := widthWorld / availableWidth
		zoomY := heightWorld / availableHeight
		requiredZoom := math.Max(zoomX, zoomY)
		
		// Update zoom and camera
		s.zoom = requiredZoom
		s.cameraX = centerWorldX
		s.cameraY = centerWorldY
	} else {
		// No triangle, just center on selected point
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
	
	// Draw the line with appropriate step size in world units
	stepSize := 0.1 // Small step in world units for smooth line
	for worldX := minWorldX; worldX <= maxWorldX; worldX += stepSize {
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

// findLargestPowerOf2 finds the largest k such that y is divisible by 2^k
// k is the number of times y is divisible by 2
func findLargestPowerOf2(y int) int {
	if y == 0 {
		return 0
	}
	k := 0
	for y%2 == 0 {
		y /= 2
		k++
	}
	return k
}

func (s *StairsChapter) drawTriangle(screen *ebiten.Image) {
	// Get the selected point in world coordinates (math space)
	_, selectedWorldY := s.getPointForIndex(s.selectedIndex)
	
	// Calculate y value in world coordinates
	yValue := int(3.0*float64(s.selectedIndex) + 1.0)
	
	// Find the largest k where y is divisible by 2^k
	k := findLargestPowerOf2(yValue)
	if k == 0 {
		return // No triangle if y is odd
	}
	
	// Calculate 2^k
	powerOf2 := math.Pow(2.0, float64(k))
	
	// The new line has slope 2^k and passes through origin (0,0) in world coordinates
	// So in world coordinates: y = 2^k * x (riseOverRun = 2^k, constant = 0)
	
	// Draw the line with slope 2^k using the drawLine function
	lineColor := color.RGBA{100, 255, 100, 255} // Green for the new line
	s.drawLine(screen, powerOf2, 0.0, lineColor)
	
	// Find intersection of horizontal line from selected point with the new line
	// In world units:
	// Horizontal line: y = selectedWorldY
	// New line: y = 2^k * x
	// So: selectedWorldY = 2^k * x
	// x = selectedWorldY / 2^k
	intersectionWorldX := selectedWorldY / powerOf2
	intersectionWorldY := selectedWorldY
	
	// Project intersection to screen coordinates
	intersectionScreenX, intersectionScreenY := s.worldToScreen(intersectionWorldX, intersectionWorldY)
	intersectionPoint := Point{
		X: intersectionScreenX,
		Y: intersectionScreenY,
	}
	
	// Get selected point in screen coordinates for drawing
	selectedPoint := s.getPointForIndexScreen(s.selectedIndex)
	
	// Draw horizontal line from selected point to intersection
	horizontalColor := color.RGBA{255, 255, 100, 255} // Yellow for horizontal
	s.drawLineSegment(screen, selectedPoint, intersectionPoint, horizontalColor)
	
	// Find the point on the original line (y = 3x + 1 in world units) at the same x as intersection
	// In world units: y = 3 * intersectionWorldX + 1
	originalWorldY := 3.0*intersectionWorldX + 1.0
	
	// Project to screen coordinates
	originalScreenX, originalScreenY := s.worldToScreen(intersectionWorldX, originalWorldY)
	originalLinePoint := Point{
		X: originalScreenX,
		Y: originalScreenY,
	}
	
	// Draw vertical line from intersection to original line
	verticalColor := color.RGBA{255, 100, 255, 255} // Magenta for vertical
	s.drawLineSegment(screen, intersectionPoint, originalLinePoint, verticalColor)
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

func (s *StairsChapter) drawIndex(screen *ebiten.Image) {
	// Calculate the mathematical y value (3x + 1) where x is the index
	yValue := 3.0*float64(s.selectedIndex) + 1.0
	yValueInt := int(yValue)
	
	// Calculate k (number of times y is divisible by 2)
	k := findLargestPowerOf2(yValueInt)
	
	// Format the display text
	indexText := "Index: " + strconv.Itoa(s.selectedIndex)
	yText := "y = " + strconv.FormatFloat(yValue, 'f', 1, 64)
	kText := "k = " + strconv.Itoa(k)
	
	// Draw index
	indexBounds := text.BoundString(basicfont.Face7x13, indexText)
	indexX := (screenWidth - indexBounds.Dx()) / 2
	indexY := screenHeight - 60
	text.Draw(screen, indexText, basicfont.Face7x13, indexX, indexY, color.White)
	
	// Draw y value
	yBounds := text.BoundString(basicfont.Face7x13, yText)
	yX := (screenWidth - yBounds.Dx()) / 2
	yY := screenHeight - 45
	text.Draw(screen, yText, basicfont.Face7x13, yX, yY, color.White)
	
	// Draw k value
	kBounds := text.BoundString(basicfont.Face7x13, kText)
	kX := (screenWidth - kBounds.Dx()) / 2
	kY := screenHeight - 30
	text.Draw(screen, kText, basicfont.Face7x13, kX, kY, color.White)
	
	// Draw additional info
	infoText := "Use Arrow Keys or A/D to navigate"
	infoBounds := text.BoundString(basicfont.Face7x13, infoText)
	infoX := (screenWidth - infoBounds.Dx()) / 2
	infoY := screenHeight - 15
	text.Draw(screen, infoText, basicfont.Face7x13, infoX, infoY, color.Gray{Y: 100})
}

