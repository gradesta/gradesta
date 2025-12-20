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
	startX        float64
	startY        float64
}

// NewStairsChapter creates a new Stairs chapter
func NewStairsChapter() *StairsChapter {
	startX := 100.0
	startY := 400.0 // Base Y position (lower on screen)
	
	return &StairsChapter{
		selectedIndex: 1, // Start at an odd index
		cameraX:        0,
		cameraY:        0,
		startX:         startX,
		startY:         startY,
	}
}

// getPointForIndex calculates the point for a given index procedurally
// The line equation is y = 3x + 1, but inverted for screen coordinates
func (s *StairsChapter) getPointForIndex(index int) Point {
	// Calculate x position based on index
	x := s.startX + float64(index)*pointSpacing
	
	// Calculate y = 3x + 1, but invert it so line goes upward on screen
	// yOffset is the mathematical y value (3x + 1)
	yOffset := 3.0*(x-s.startX) + 1.0
	y := s.startY - yOffset // Subtract to go upward on screen
	
	return Point{
		X:     x,
		Y:     y,
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
	// Center the selected point on screen
	selectedPoint := s.getPointForIndex(s.selectedIndex)
	s.cameraX = selectedPoint.X - screenWidth/2
	s.cameraY = selectedPoint.Y - screenHeight/2
}

func (s *StairsChapter) Draw(screen *ebiten.Image) {
	// Draw axes at point 0
	s.drawAxes(screen)
	
	// Draw the line with slope 3x+1 procedurally
	s.drawLine(screen)
	
	// Draw visible points procedurally
	s.drawPoints(screen)
	
	// Draw selected point index and y value at the bottom
	s.drawIndex(screen)
}

func (s *StairsChapter) drawAxes(screen *ebiten.Image) {
	// Get point 0 (origin)
	origin := s.getPointForIndex(0)
	
	// Transform origin to screen coordinates
	originX := origin.X - s.cameraX
	originY := origin.Y - s.cameraY
	
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

func (s *StairsChapter) drawLine(screen *ebiten.Image) {
	// Calculate the visible range of x coordinates on screen
	minScreenX := s.cameraX - 100 // Add padding
	maxScreenX := s.cameraX + float64(screenWidth) + 100
	
	// Calculate which indices correspond to visible x range
	// x = startX + index * pointSpacing
	minIndex := int((minScreenX - s.startX) / pointSpacing) - 1
	maxIndex := int((maxScreenX - s.startX) / pointSpacing) + 1
	
	// Draw line segments between consecutive visible points
	for i := minIndex; i < maxIndex; i++ {
		p1 := s.getPointForIndex(i)
		p2 := s.getPointForIndex(i + 1)
		
		// Transform points to screen coordinates (accounting for camera)
		x1 := p1.X - s.cameraX
		y1 := p1.Y - s.cameraY
		x2 := p2.X - s.cameraX
		y2 := p2.Y - s.cameraY
		
		// Only draw if at least part of the segment is on screen
		if (x1 >= -50 && x1 < float64(screenWidth)+50) || (x2 >= -50 && x2 < float64(screenWidth)+50) ||
			(y1 >= -50 && y1 < float64(screenHeight)+50) || (y2 >= -50 && y2 < float64(screenHeight)+50) {
			
			// Draw a simple line using multiple small rectangles
			dx := x2 - x1
			dy := y2 - y1
			length := math.Sqrt(dx*dx + dy*dy)
			steps := int(length)
			
			if steps > 0 {
				stepX := dx / float64(steps)
				stepY := dy / float64(steps)
				
				for j := 0; j < steps; j++ {
					x := x1 + float64(j)*stepX
					y := y1 + float64(j)*stepY
					// Only draw if on screen
					if x >= 0 && x < float64(screenWidth) && y >= 0 && y < float64(screenHeight) {
						ebitenutil.DrawRect(screen, x-0.5, y-0.5, 1, 1, color.Gray{Y: 100})
					}
				}
			}
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

