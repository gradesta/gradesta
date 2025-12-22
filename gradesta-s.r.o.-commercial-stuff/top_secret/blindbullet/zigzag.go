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
	zigzagBufferSize = 35 // Number of previous points to keep
	zigzagZigWidth   = 30.0 // Width of zig movement
	zigzagZagWidth   = 30.0 // Width of zag movement
)

// ZigZagPoint represents a point in the zigzag path
type ZigZagPoint struct {
	Index *big.Int // The index value
	Y     *big.Int // The Y value (coefficient * index + 1)
	Move  int      // +1 for right (zig), -1 for left (zag)
}

// ZigZagChapter implements the N-Dimensional Zig Zag chapter
type ZigZagChapter struct {
	currentIndex *big.Int // Current index (starts at -1)
	globalCoeficient float64 // Coefficient for the line equation (y = globalCoeficient*x + 1)
	
	// Buffer of previous points (most recent at end)
	pointBuffer []ZigZagPoint
	
	
	// Cached staircase calculation
	cachedSteps      []StairStep
	cachedIsUpwards  bool
	cachedHitLimit   bool
	cachedForIndex   *big.Int
	cachedForCoef    float64
	
	// Jump input mode
	jumpInputMode bool   // Whether we're in jump input mode
	jumpInputBuffer string // Buffer for typing step number
	
	escConsumed bool // Whether ESC was consumed by a dialog this frame
}

// NewZigZagChapter creates a new Zig Zag chapter
func NewZigZagChapter() *ZigZagChapter {
	zz := &ZigZagChapter{
		currentIndex:    big.NewInt(-1),
		globalCoeficient: 3.0, // y = 3x + 1
		pointBuffer:     make([]ZigZagPoint, 0, zigzagBufferSize),
		cachedForIndex:  nil,
	}
	
	// Initialize with starting point
	zz.addPointToBuffer(zz.currentIndex, 0) // 0 indicates initial point (no move)
	
	return zz
}

// addPointToBuffer adds a point to the buffer, maintaining max size
func (z *ZigZagChapter) addPointToBuffer(index *big.Int, move int) {
	// Calculate Y value
	coefBig := big.NewInt(int64(z.globalCoeficient))
	yValueBig := new(big.Int).Mul(coefBig, index)
	yValueBig.Add(yValueBig, big.NewInt(1))
	
	point := ZigZagPoint{
		Index: new(big.Int).Set(index),
		Y:     yValueBig,
		Move:  move,
	}
	
	z.pointBuffer = append(z.pointBuffer, point)
	
	// Keep only the last zigzagBufferSize points
	if len(z.pointBuffer) > zigzagBufferSize {
		z.pointBuffer = z.pointBuffer[len(z.pointBuffer)-zigzagBufferSize:]
	}
}

func (z *ZigZagChapter) Update() error {
	z.escConsumed = false // Reset at start of frame
	
	// Handle 'r' key to reset to index -1
	if inpututil.IsKeyJustPressed(ebiten.KeyR) {
		z.currentIndex.SetInt64(-1)
		z.pointBuffer = make([]ZigZagPoint, 0, zigzagBufferSize)
		z.addPointToBuffer(z.currentIndex, 0) // Reset with initial point
		z.cachedForIndex = nil // Invalidate cache
		return nil
	}
	
	// Handle 'j' key to enter jump input mode
	if inpututil.IsKeyJustPressed(ebiten.KeyJ) {
		z.jumpInputMode = true
		z.jumpInputBuffer = ""
		return nil
	}
	
	// Handle jump input mode
	if z.jumpInputMode {
		// Handle Enter to confirm jump
		if inpututil.IsKeyJustPressed(ebiten.KeyEnter) {
			// Parse the input as a big.Int
			if z.jumpInputBuffer != "" {
				newIndex := new(big.Int)
				if _, ok := newIndex.SetString(z.jumpInputBuffer, 10); ok {
					z.currentIndex.Set(newIndex)
					z.pointBuffer = make([]ZigZagPoint, 0, zigzagBufferSize)
					z.addPointToBuffer(z.currentIndex, 0) // Add new starting point
					z.cachedForIndex = nil // Invalidate cache
				}
			}
			z.jumpInputMode = false
			z.jumpInputBuffer = ""
		}
		
		// Handle Escape to cancel
		if inpututil.IsKeyJustPressed(ebiten.KeyEscape) {
			z.jumpInputMode = false
			z.jumpInputBuffer = ""
			z.escConsumed = true
		}
		
		// Handle Backspace
		if inpututil.IsKeyJustPressed(ebiten.KeyBackspace) {
			if len(z.jumpInputBuffer) > 0 {
				z.jumpInputBuffer = z.jumpInputBuffer[:len(z.jumpInputBuffer)-1]
			}
		}
		
		// Handle numeric input and minus sign
		for i := ebiten.Key0; i <= ebiten.Key9; i++ {
			if inpututil.IsKeyJustPressed(i) {
				digit := '0' + (i - ebiten.Key0)
				z.jumpInputBuffer += string(digit)
			}
		}
		if inpututil.IsKeyJustPressed(ebiten.KeyMinus) {
			if z.jumpInputBuffer == "" || z.jumpInputBuffer[0] != '-' {
				z.jumpInputBuffer = "-" + z.jumpInputBuffer
			}
		}
		
		// Don't process other keys when in jump input mode
		return nil
	}
	
	// Handle left arrow: move by +prev-prev Y and remove from buffer
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowLeft) || inpututil.IsKeyJustPressed(ebiten.KeyA) {
		// Remove the last point from buffer if there are points
		if len(z.pointBuffer) > 1 {
			// Remove the last point
			z.pointBuffer = z.pointBuffer[:len(z.pointBuffer)-1]
			// Set current index to the new last point
			z.currentIndex.Set(z.pointBuffer[len(z.pointBuffer)-1].Index)
			z.cachedForIndex = nil // Invalidate cache
			return nil
		}
		
		// If only one point or empty, get prev-prev Y value (from two points ago in buffer)
		var prevPrevY *big.Int
		if len(z.pointBuffer) >= 2 {
			// Get Y from two points ago
			prevPrevY = z.pointBuffer[len(z.pointBuffer)-2].Y
		} else if len(z.pointBuffer) == 1 {
			// If only one previous point, use its Y
			prevPrevY = z.pointBuffer[0].Y
		} else {
			// If no previous points, calculate Y for current index
			coefBig := big.NewInt(int64(z.globalCoeficient))
			prevPrevY = new(big.Int).Mul(coefBig, z.currentIndex)
			prevPrevY.Add(prevPrevY, big.NewInt(1))
		}
		
		// Move by +prevPrevY
		newIndex := new(big.Int).Add(z.currentIndex, prevPrevY)
		
		// Add point to buffer with zag (left) move
		z.addPointToBuffer(newIndex, -1)
		z.currentIndex.Set(newIndex)
		z.cachedForIndex = nil // Invalidate cache
	}
	
	// Handle right arrow: move by -prevY
	if inpututil.IsKeyJustPressed(ebiten.KeyArrowRight) || inpututil.IsKeyJustPressed(ebiten.KeyD) {
		// Get previous Y value (from the last point in buffer)
		var previousY *big.Int
		if len(z.pointBuffer) > 0 {
			previousY = z.pointBuffer[len(z.pointBuffer)-1].Y
		} else {
			// If no previous point, calculate Y for current index
			coefBig := big.NewInt(int64(z.globalCoeficient))
			previousY = new(big.Int).Mul(coefBig, z.currentIndex)
			previousY.Add(previousY, big.NewInt(1))
		}
		
		// Move by -previousY
		negY := new(big.Int).Neg(previousY)
		newIndex := new(big.Int).Add(z.currentIndex, negY)
		
		// Add point to buffer with zig (right) move
		z.addPointToBuffer(newIndex, 1)
		z.currentIndex.Set(newIndex)
		z.cachedForIndex = nil // Invalidate cache
	}
	
	return nil
}

func (z *ZigZagChapter) Draw(screen *ebiten.Image) {
	screen.Fill(color.RGBA{20, 20, 30, 255})
	
	// Draw the zigzag visualization
	z.drawZigZag(screen)
	
	// Draw the Collatz table
	z.drawCollatzTable(screen)
	
	// Draw current index info
	z.drawCurrentInfo(screen)
	
	// Draw jump input dialog if in jump input mode
	if z.jumpInputMode {
		z.drawJumpInput(screen)
	}
}

// drawZigZag draws a line graph on a virtual plane angled away from the viewer
func (z *ZigZagChapter) drawZigZag(screen *ebiten.Image) {
	if len(z.pointBuffer) < 1 {
		return // Need at least 1 point
	}
	
	// Calculate min and max Y values for scaling
	minY, maxY := z.calculateYBounds()
	
	// Calculate scale for Y values (how many pixels per unit)
	// We want to fit the range on screen
	yRange := math.Max(math.Abs(minY), math.Abs(maxY))
	if yRange == 0 {
		yRange = 1
	}
	scaleY := 200.0 / yRange // Scale to fit within 200 pixels on each side of zero
	
	// The plane is hinged at the center of the bottom edge of the screen
	// Zero axis is at the bottom center, plane opens upward and away from viewer
	// Use more vertical space - start from lower third instead of bottom
	bottomY := float64(screenHeight) - 100.0 // Bottom edge with padding
	topY := 100.0 // Top edge with padding
	centerX := float64(screenWidth) / 2
	verticalRange := bottomY - topY // Available vertical space
	
	// Plane angle (70 degrees from vertical, opening straight away from viewer)
	planeAngle := 70.0 * math.Pi / 180.0
	sinAngle := math.Sin(planeAngle)
	
	// Perspective parameters
	depthScale := 0.15 // How much depth affects position
	
	// Draw the zero axis first (fixed line at bottom center of screen, Y=0)
	// The axis extends horizontally from the center
	zeroAxisLength := 400.0 // Fixed length for the axis
	zeroAxisStartX := centerX - zeroAxisLength/2
	zeroAxisEndX := centerX + zeroAxisLength/2
	
	// Zero axis is at the bottom center (hinge point)
	zeroAxisY := bottomY
	
	// Draw zero axis line (horizontal line at bottom center)
	ebitenutil.DrawLine(screen, zeroAxisStartX, zeroAxisY, zeroAxisEndX, zeroAxisY, color.RGBA{100, 100, 100, 255})
	
	// Convert points to screen coordinates on the angled plane
	screenPoints := make([]struct {
		x, y float64
		value float64
		alpha float64
	}, len(z.pointBuffer))
	
	for i, point := range z.pointBuffer {
		// Convert Y to float64
		yFloat := new(big.Float).SetInt(point.Y)
		yFloat64, _ := yFloat.Float64()
		
		// Position on the plane:
		// - Horizontal position: based on Y value (negative = left, positive = right, centered)
		// - Depth: based on buffer index (later points are farther forward, closer to viewer)
		
		// Horizontal offset from center: based on Y value
		// Negative Y values go left, positive Y values go right
		horizontalOffset := yFloat64 * scaleY
		
		// Depth: based on buffer index (later points are closer to viewer, less depth)
		// The newest point (last in buffer) should be closest to viewer
		baseDepthIndex := float64(i)
		
		normalizedDepth := baseDepthIndex / float64(zigzagBufferSize)
		// Clamp normalized depth to prevent negative
		if normalizedDepth < 0 {
			normalizedDepth = 0
		}
		depth := normalizedDepth * verticalRange * 0.8 // Use most of the vertical range
		perspectiveScale := 1.0 + (1.0 - normalizedDepth) * depthScale // Later points are larger (closer)
		
		// No animation
		alpha := 1.0
		
		// Project onto angled plane
		// The plane opens straight upward and away from the center bottom
		// - X: centered, horizontal offset based on Y value
		// - Y: moves up from bottom based on depth
		projX := centerX + horizontalOffset * perspectiveScale
		// Y position: start at bottom (zero axis), move up based on depth
		// The plane tilts straight up, so depth moves the point upward
		projY := zeroAxisY - depth*sinAngle
		
		screenX := projX
		screenY := projY
		
		// Store alpha for drawing
		screenPoints[i] = struct {
			x, y float64
			value float64
			alpha float64
		}{screenX, screenY, yFloat64, alpha}
	}
	
	// Draw lines connecting points (line graph)
	if len(screenPoints) > 1 {
		for i := 0; i < len(screenPoints)-1; i++ {
			p1 := screenPoints[i]
			p2 := screenPoints[i+1]
			
			// Line color based on direction
			var lineColor color.RGBA
			if i < len(z.pointBuffer)-1 {
				if z.pointBuffer[i+1].Move > 0 {
					lineColor = color.RGBA{100, 255, 100, 255} // Green for right
				} else if z.pointBuffer[i+1].Move < 0 {
					lineColor = color.RGBA{255, 100, 100, 255} // Red for left
				} else {
					lineColor = color.RGBA{150, 200, 255, 255} // Default blue
				}
			} else {
				lineColor = color.RGBA{150, 200, 255, 255} // Default blue
			}
			
			// Apply alpha to line color (use minimum of both points)
			lineAlpha := math.Min(p1.alpha, p2.alpha)
			if lineAlpha < 1.0 {
				// Adjust alpha for line
				lineColor = color.RGBA{
					uint8(float64(lineColor.R) * lineAlpha),
					uint8(float64(lineColor.G) * lineAlpha),
					uint8(float64(lineColor.B) * lineAlpha),
					uint8(float64(lineColor.A) * lineAlpha),
				}
			}
			
			ebitenutil.DrawLine(screen, p1.x, p1.y, p2.x, p2.y, lineColor)
		}
	}
	
	// Draw points
	for i, sp := range screenPoints {
		pointColor := color.RGBA{200, 200, 200, 255}
		if i == len(screenPoints)-1 {
			// Current point is highlighted
			pointColor = color.RGBA{255, 255, 100, 255}
		}
		
		// Apply alpha
		if sp.alpha < 1.0 {
			pointColor = color.RGBA{
				uint8(float64(pointColor.R) * sp.alpha),
				uint8(float64(pointColor.G) * sp.alpha),
				uint8(float64(pointColor.B) * sp.alpha),
				uint8(float64(pointColor.A) * sp.alpha),
			}
		}
		
		// Point size based on depth (closer points larger, farther points smaller)
		baseDepthIndex := float64(i)
		normalizedDepth := baseDepthIndex / float64(zigzagBufferSize)
		if normalizedDepth < 0 {
			normalizedDepth = 0
		}
		pointSize := 4.0 + (1.0 - normalizedDepth) * 4.0 // Closer points (later in buffer) are larger
		if i == len(screenPoints)-1 {
			pointSize += 2.0 // Current point is even larger
		}
		
		ebitenutil.DrawRect(screen, sp.x-pointSize/2, sp.y-pointSize/2, pointSize, pointSize, pointColor)
	}
}

// calculateYBounds calculates the min and max Y values in the buffer
func (z *ZigZagChapter) calculateYBounds() (minY, maxY float64) {
	if len(z.pointBuffer) == 0 {
		return 0, 0
	}
	
	// Initialize with first point
	firstYFloat := new(big.Float).SetInt(z.pointBuffer[0].Y)
	firstYFloat64, _ := firstYFloat.Float64()
	
	minY = firstYFloat64
	maxY = firstYFloat64
	
	// Find min/max Y
	for _, point := range z.pointBuffer {
		yFloat := new(big.Float).SetInt(point.Y)
		yFloat64, _ := yFloat.Float64()
		
		if yFloat64 < minY {
			minY = yFloat64
		}
		if yFloat64 > maxY {
			maxY = yFloat64
		}
	}
	
	return minY, maxY
}

// drawCollatzTable draws the Collatz table for the current index
func (z *ZigZagChapter) drawCollatzTable(screen *ebiten.Image) {
	steps, _, _ := z.getCachedStaircase()
	
	data := CollatzTableData{
		Steps:      steps,
		StartIndex: new(big.Int).Set(z.currentIndex),
		Coefficient: z.globalCoeficient,
		IsUpwards:  false, // Not used for zigzag
		HitLimit:   false, // Not used for zigzag
		StopOnDirectionChange: true,
	}
	DrawCollatzTable(screen, data)
}

// getCachedStaircase returns the cached staircase or calculates it if cache is invalid
func (z *ZigZagChapter) getCachedStaircase() ([]StairStep, bool, bool) {
	// Check if cache is valid
	cacheValid := false
	if z.cachedForIndex != nil {
		cacheValid = z.cachedForIndex.Cmp(z.currentIndex) == 0 && z.cachedForCoef == z.globalCoeficient
	}
	if cacheValid {
		return z.cachedSteps, z.cachedIsUpwards, z.cachedHitLimit
	}
	
	// Cache is invalid, recalculate
	maxSteps := 50
	steps, isUpwards, hitLimit := CalculateStaircase(z.currentIndex, z.globalCoeficient, maxSteps, true)
	
	// Update cache
	z.cachedSteps = steps
	z.cachedIsUpwards = isUpwards
	z.cachedHitLimit = hitLimit
	z.cachedForIndex = new(big.Int).Set(z.currentIndex)
	z.cachedForCoef = z.globalCoeficient
	
	return steps, isUpwards, hitLimit
}

// drawCurrentInfo draws information about the current point
func (z *ZigZagChapter) drawCurrentInfo(screen *ebiten.Image) {
	// Draw current index at bottom
	indexText := "Index: " + z.currentIndex.String()
	text.Draw(screen, indexText, basicfont.Face7x13, 10, screenHeight-30, color.White)
	
	// Draw controls
	controlsText := "LEFT: +prev-prevY | RIGHT: -prevY | R: reset | J: jump | ESC: back"
	text.Draw(screen, controlsText, basicfont.Face7x13, 10, screenHeight-15, color.Gray{Y: 150})
}

// drawJumpInput draws the jump input dialog
func (z *ZigZagChapter) drawJumpInput(screen *ebiten.Image) {
	// Draw a semi-transparent overlay
	overlayColor := color.RGBA{0, 0, 0, 200}
	ebitenutil.DrawRect(screen, 0, 0, float64(screenWidth), float64(screenHeight), overlayColor)
	
	// Draw dialog box
	dialogWidth := 400.0
	dialogHeight := 100.0
	dialogX := (float64(screenWidth) - dialogWidth) / 2
	dialogY := (float64(screenHeight) - dialogHeight) / 2
	
	// Draw dialog background
	ebitenutil.DrawRect(screen, dialogX, dialogY, dialogWidth, dialogHeight, color.RGBA{40, 40, 50, 255})
	
	// Draw dialog border
	ebitenutil.DrawRect(screen, dialogX, dialogY, dialogWidth, 2, color.White)
	ebitenutil.DrawRect(screen, dialogX, dialogY, 2, dialogHeight, color.White)
	ebitenutil.DrawRect(screen, dialogX+dialogWidth-2, dialogY, 2, dialogHeight, color.White)
	ebitenutil.DrawRect(screen, dialogX, dialogY+dialogHeight-2, dialogWidth, 2, color.White)
	
	// Draw prompt
	promptText := "Enter index to jump to:"
	text.Draw(screen, promptText, basicfont.Face7x13, int(dialogX)+10, int(dialogY)+20, color.White)
	
	// Draw input buffer
	inputText := z.jumpInputBuffer
	if inputText == "" {
		inputText = "0"
	}
	text.Draw(screen, inputText, basicfont.Face7x13, int(dialogX)+10, int(dialogY)+40, color.RGBA{100, 255, 100, 255})
	
	// Draw instructions
	instText := "Press ENTER to confirm, ESC to cancel"
	text.Draw(screen, instText, basicfont.Face7x13, int(dialogX)+10, int(dialogY)+70, color.Gray{Y: 150})
}

// WasEscConsumed returns whether ESC was consumed this frame
func (z *ZigZagChapter) WasEscConsumed() bool {
	return z.escConsumed
}

