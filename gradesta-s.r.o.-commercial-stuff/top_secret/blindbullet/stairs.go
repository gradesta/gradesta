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
	numPoints = 20
	pointSize = 8
)

// Point represents a point on the 45-degree line
type Point struct {
	X, Y float64
}

// StairsChapter implements the Stairs chapter
type StairsChapter struct {
	points        []Point
	selectedIndex int
	cameraX       float64
	cameraY       float64
}

// NewStairsChapter creates a new Stairs chapter
func NewStairsChapter() *StairsChapter {
	points := make([]Point, numPoints)
	
	// Create points along a 45-degree line
	// Starting from top-left, going to bottom-right
	startX := 100.0
	startY := 100.0
	lineLength := 500.0
	
	for i := 0; i < numPoints; i++ {
		t := float64(i) / float64(numPoints-1)
		points[i] = Point{
			X: startX + t*lineLength,
			Y: startY + t*lineLength, // 45 degrees means equal X and Y change
		}
	}
	
	return &StairsChapter{
		points:        points,
		selectedIndex: 0,
		cameraX:       0,
		cameraY:       0,
	}
}

func (s *StairsChapter) Update() error {
	// Handle arrow key input
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowRight) || inpututil.IsKeyJustPressed(ebiten.KeyD) {
		s.selectedIndex = (s.selectedIndex + 1) % numPoints
		s.updateCamera()
	}
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowLeft) || inpututil.IsKeyJustPressed(ebiten.KeyA) {
		s.selectedIndex = (s.selectedIndex - 1 + numPoints) % numPoints
		s.updateCamera()
	}
	
	return nil
}

func (s *StairsChapter) updateCamera() {
	// Center the selected point on screen
	selectedPoint := s.points[s.selectedIndex]
	s.cameraX = selectedPoint.X - screenWidth/2
	s.cameraY = selectedPoint.Y - screenHeight/2
}

func (s *StairsChapter) Draw(screen *ebiten.Image) {
	// Draw the 45-degree line
	s.drawLine(screen)
	
	// Draw all points
	for i, point := range s.points {
		s.drawPoint(screen, point, i == s.selectedIndex)
	}
	
	// Draw selected point index at the bottom
	s.drawIndex(screen)
}

func (s *StairsChapter) drawLine(screen *ebiten.Image) {
	if len(s.points) < 2 {
		return
	}
	
	// Draw line segments between consecutive points
	for i := 0; i < len(s.points)-1; i++ {
		p1 := s.points[i]
		p2 := s.points[i+1]
		
		// Transform points to screen coordinates (accounting for camera)
		x1 := p1.X - s.cameraX
		y1 := p1.Y - s.cameraY
		x2 := p2.X - s.cameraX
		y2 := p2.Y - s.cameraY
		
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
				ebitenutil.DrawRect(screen, x-0.5, y-0.5, 1, 1, color.Gray{Y: 100})
			}
		}
	}
}

func (s *StairsChapter) drawPoint(screen *ebiten.Image, point Point, selected bool) {
	// Transform point to screen coordinates
	x := point.X - s.cameraX
	y := point.Y - s.cameraY
	
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
	indexText := "Index: " + strconv.Itoa(s.selectedIndex)
	textBounds := text.BoundString(basicfont.Face7x13, indexText)
	
	// Center the text at the bottom
	x := (screenWidth - textBounds.Dx()) / 2
	y := screenHeight - 30
	
	text.Draw(screen, indexText, basicfont.Face7x13, x, y, color.White)
	
	// Draw additional info
	infoText := "Use Arrow Keys or A/D to navigate"
	infoBounds := text.BoundString(basicfont.Face7x13, infoText)
	infoX := (screenWidth - infoBounds.Dx()) / 2
	infoY := screenHeight - 15
	text.Draw(screen, infoText, basicfont.Face7x13, infoX, infoY, color.Gray{Y: 100})
}

