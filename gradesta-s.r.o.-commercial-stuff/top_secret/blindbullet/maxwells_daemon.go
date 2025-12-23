package main

import (
	"image/color"
	"math"
	"math/rand"
	"strconv"
	"time"

	"github.com/hajimehoshi/ebiten/v2"
	"github.com/hajimehoshi/ebiten/v2/ebitenutil"
	"github.com/hajimehoshi/ebiten/v2/inpututil"
	"github.com/hajimehoshi/ebiten/v2/text"
	"golang.org/x/image/font/basicfont"
)

// Particle represents a bouncing particle
type Particle struct {
	X, Y   float64 // Position
	VX, VY float64 // Velocity
	Radius float64 // Particle radius
}

// MaxwellsDaemonChapter implements the Maxwell's Daemon chapter
type MaxwellsDaemonChapter struct {
	particles    []Particle
	doorOpen     bool
	level        int
	particleCount int
	
	// Box dimensions
	boxX, boxY      float64 // Top-left corner of box
	boxWidth        float64
	boxHeight       float64
	
	// Wall dimensions
	wallX           float64 // X position of wall (middle of screen)
	wallTopY        float64
	wallBottomY     float64
	doorHeight      float64 // Height of door opening
	doorCenterY     float64 // Center Y of door
	
	// Physics
	maxVelocity     float64
	particleRadius  float64
	
	// Win state
	won             bool
	winTime         time.Time
	
	escConsumed     bool // Whether ESC was consumed by a dialog this frame
}

// NewMaxwellsDaemonChapter creates a new Maxwell's Daemon chapter
func NewMaxwellsDaemonChapter() *MaxwellsDaemonChapter {
	m := &MaxwellsDaemonChapter{
		boxX:           50,
		boxY:           50,
		boxWidth:       700,
		boxHeight:      500,
		maxVelocity:    5.0, // Increased from 3.0 for more fun
		particleRadius: 5.0,
		doorHeight:     80.0,
		level:          1,
		particleCount:  6,
	}
	
	m.wallX = m.boxX + m.boxWidth/2
	m.wallTopY = m.boxY
	m.wallBottomY = m.boxY + m.boxHeight
	m.doorCenterY = m.boxY + m.boxHeight/2
	
	m.initParticles()
	
	return m
}

// initParticles initializes particles for the current level
func (m *MaxwellsDaemonChapter) initParticles() {
	m.particles = make([]Particle, m.particleCount)
	rand.Seed(time.Now().UnixNano())
	
	leftCount := m.particleCount / 2
	
	// Initialize particles on left side
	for i := 0; i < leftCount; i++ {
		m.particles[i] = Particle{
			X:      m.boxX + 50 + rand.Float64()*(m.wallX-m.boxX-100),
			Y:      m.boxY + 50 + rand.Float64()*(m.boxHeight-100),
			VX:     (rand.Float64()*2 - 1) * m.maxVelocity,
			VY:     (rand.Float64()*2 - 1) * m.maxVelocity,
			Radius: m.particleRadius,
		}
	}
	
	// Initialize particles on right side
	for i := leftCount; i < m.particleCount; i++ {
		m.particles[i] = Particle{
			X:      m.wallX + 50 + rand.Float64()*(m.boxX+m.boxWidth-m.wallX-100),
			Y:      m.boxY + 50 + rand.Float64()*(m.boxHeight-100),
			VX:     (rand.Float64()*2 - 1) * m.maxVelocity,
			VY:     (rand.Float64()*2 - 1) * m.maxVelocity,
			Radius: m.particleRadius,
		}
	}
	
	m.won = false
}

// Update updates the Maxwell's Daemon chapter
func (m *MaxwellsDaemonChapter) Update() error {
	m.escConsumed = false // Reset at start of frame
	
	// Handle spacebar to toggle door
	if inpututil.IsKeyJustPressed(ebiten.KeySpace) {
		m.doorOpen = !m.doorOpen
	}
	
	// If won, wait a bit then advance to next level
	if m.won {
		if time.Since(m.winTime) > 2*time.Second {
			m.level++
			m.particleCount *= 2
			m.initParticles()
		}
		return nil
	}
	
	// Update particle physics
	for i := range m.particles {
		p := &m.particles[i]
		
		// Store previous position for continuous collision detection
		prevX := p.X
		prevY := p.Y
		
		// Check collisions with other particles first
		for j := i + 1; j < len(m.particles); j++ {
			other := &m.particles[j]
			
			// Calculate distance between particles
			dx := p.X - other.X
			dy := p.Y - other.Y
			distance := math.Sqrt(dx*dx + dy*dy)
			minDistance := p.Radius + other.Radius
			
			if distance < minDistance && distance > 0 {
				// Collision detected - particles are overlapping
				// Normalize collision vector
				normalX := dx / distance
				normalY := dy / distance
				
				// Separate particles to prevent overlap
				overlap := minDistance - distance
				separationX := normalX * overlap * 0.5
				separationY := normalY * overlap * 0.5
				p.X += separationX
				p.Y += separationY
				other.X -= separationX
				other.Y -= separationY
				
				// Calculate relative velocity
				relVX := p.VX - other.VX
				relVY := p.VY - other.VY
				
				// Calculate relative velocity along collision normal
				dotProduct := relVX*normalX + relVY*normalY
				
				// Only resolve if particles are moving towards each other
				if dotProduct < 0 {
					// Store speeds BEFORE collision to ensure perfect energy conservation
					pSpeedBefore := math.Sqrt(p.VX*p.VX + p.VY*p.VY)
					otherSpeedBefore := math.Sqrt(other.VX*other.VX + other.VY*other.VY)
					
					// Calculate impulse (assuming equal mass and perfectly elastic collision)
					// For elastic collision, impulse = 2 * relative_velocity_along_normal
					impulse := 2.0 * dotProduct
					
					// Update velocities (perfectly elastic collision - conserves energy)
					p.VX -= impulse * normalX
					p.VY -= impulse * normalY
					other.VX += impulse * normalX
					other.VY += impulse * normalY
					
					// Apply small angle perturbation to prevent infinite cycles
					// Use the ORIGINAL speed to ensure no energy is added
					if pSpeedBefore > 0 {
						anglePerturbation := (rand.Float64()*2.0 - 1.0) * 0.05 // ±5% of 1 radian
						currentAngle := math.Atan2(p.VY, p.VX)
						newAngle := currentAngle + anglePerturbation
						// Use original speed, not current speed (to prevent energy gain from floating point errors)
						p.VX = pSpeedBefore * math.Cos(newAngle)
						p.VY = pSpeedBefore * math.Sin(newAngle)
					}
					
					if otherSpeedBefore > 0 {
						anglePerturbation := (rand.Float64()*2.0 - 1.0) * 0.05
						currentAngle := math.Atan2(other.VY, other.VX)
						newAngle := currentAngle + anglePerturbation
						// Use original speed, not current speed (to prevent energy gain from floating point errors)
						other.VX = otherSpeedBefore * math.Cos(newAngle)
						other.VY = otherSpeedBefore * math.Sin(newAngle)
					}
				}
			}
		}
		
		// Update position
		p.X += p.VX
		p.Y += p.VY
		
		// Collision with top/bottom walls (continuous collision detection)
		if p.Y-p.Radius < m.boxY {
			// Check if particle crossed the boundary this frame
			if prevY-p.Radius >= m.boxY {
				// Particle crossed top wall - place at boundary
				p.Y = m.boxY + p.Radius
				// Reflect Y component and add small angle perturbation (preserving magnitude)
				speed := math.Sqrt(p.VX*p.VX + p.VY*p.VY)
				anglePerturbation := (rand.Float64()*2.0 - 1.0) * 0.05 // ±5% of 1 radian
				// Reflect and perturb angle
				reflectedAngle := math.Atan2(-p.VY, p.VX) + anglePerturbation
				p.VX = speed * math.Cos(reflectedAngle)
				p.VY = speed * math.Sin(reflectedAngle)
			} else {
				// Particle was already past boundary, just correct position
				p.Y = m.boxY + p.Radius
			}
		}
		if p.Y+p.Radius > m.boxY+m.boxHeight {
			// Check if particle crossed the boundary this frame
			if prevY+p.Radius <= m.boxY+m.boxHeight {
				// Particle crossed bottom wall - place at boundary
				p.Y = m.boxY + m.boxHeight - p.Radius
				// Reflect Y component and add small angle perturbation (preserving magnitude)
				speed := math.Sqrt(p.VX*p.VX + p.VY*p.VY)
				anglePerturbation := (rand.Float64()*2.0 - 1.0) * 0.05
				// Reflect and perturb angle
				reflectedAngle := math.Atan2(-p.VY, p.VX) + anglePerturbation
				p.VX = speed * math.Cos(reflectedAngle)
				p.VY = speed * math.Sin(reflectedAngle)
			} else {
				// Particle was already past boundary, just correct position
				p.Y = m.boxY + m.boxHeight - p.Radius
			}
		}
		
		// Collision with left wall (continuous collision detection)
		if p.X-p.Radius < m.boxX {
			// Check if particle crossed the boundary this frame
			if prevX-p.Radius >= m.boxX {
				// Particle crossed left wall - place at boundary
				p.X = m.boxX + p.Radius
				// Reflect X component and add small angle perturbation (preserving magnitude)
				speed := math.Sqrt(p.VX*p.VX + p.VY*p.VY)
				anglePerturbation := (rand.Float64()*2.0 - 1.0) * 0.05
				// Reflect and perturb angle
				reflectedAngle := math.Atan2(p.VY, -p.VX) + anglePerturbation
				p.VX = speed * math.Cos(reflectedAngle)
				p.VY = speed * math.Sin(reflectedAngle)
			} else {
				// Particle was already past boundary, just correct position
				p.X = m.boxX + p.Radius
			}
		}
		
		// Collision with right wall (continuous collision detection)
		if p.X+p.Radius > m.boxX+m.boxWidth {
			// Check if particle crossed the boundary this frame
			if prevX+p.Radius <= m.boxX+m.boxWidth {
				// Particle crossed right wall - place at boundary
				p.X = m.boxX + m.boxWidth - p.Radius
				// Reflect X component and add small angle perturbation (preserving magnitude)
				speed := math.Sqrt(p.VX*p.VX + p.VY*p.VY)
				anglePerturbation := (rand.Float64()*2.0 - 1.0) * 0.05
				// Reflect and perturb angle
				reflectedAngle := math.Atan2(p.VY, -p.VX) + anglePerturbation
				p.VX = speed * math.Cos(reflectedAngle)
				p.VY = speed * math.Sin(reflectedAngle)
			} else {
				// Particle was already past boundary, just correct position
				p.X = m.boxX + m.boxWidth - p.Radius
			}
		}
		
		// Collision with middle wall (continuous collision detection)
		doorTop := m.doorCenterY - m.doorHeight/2
		doorBottom := m.doorCenterY + m.doorHeight/2
		
		// Check if particle crossed the wall boundary this frame
		prevLeft := prevX - p.Radius
		prevRight := prevX + p.Radius
		currLeft := p.X - p.Radius
		currRight := p.X + p.Radius
		
		// Check if particle crossed from left to right or right to left
		crossedFromLeft := prevRight <= m.wallX && currRight > m.wallX
		crossedFromRight := prevLeft >= m.wallX && currLeft < m.wallX
		
		if crossedFromLeft || crossedFromRight {
			// Calculate intersection point with wall
			var intersectionY float64
			if crossedFromLeft {
				// Calculate Y where particle crossed the wall
				t := (m.wallX - prevRight) / (currRight - prevRight)
				intersectionY = prevY + t*(p.Y-prevY)
			} else {
				// Calculate Y where particle crossed the wall
				t := (m.wallX - prevLeft) / (currLeft - prevLeft)
				intersectionY = prevY + t*(p.Y-prevY)
			}
			
			inDoorOpening := intersectionY >= doorTop && intersectionY <= doorBottom
			
			if m.doorOpen {
				// Green bar visible = door is CLOSED - always bounce (block all particles)
				// Place particle at wall boundary
				if crossedFromLeft {
					p.X = m.wallX - p.Radius
				} else {
					p.X = m.wallX + p.Radius
				}
				// Reflect X component and add small angle perturbation (preserving magnitude)
				speed := math.Sqrt(p.VX*p.VX + p.VY*p.VY)
				anglePerturbation := (rand.Float64()*2.0 - 1.0) * 0.05
				// Reflect and perturb angle
				reflectedAngle := math.Atan2(p.VY, -p.VX) + anglePerturbation
				p.VX = speed * math.Cos(reflectedAngle)
				p.VY = speed * math.Sin(reflectedAngle)
			} else {
				// No green bar = door is OPEN - particles can pass through if in door opening
				if !inDoorOpening {
					// Hit the wall (above or below door) - bounce
					// Place particle at wall boundary
					if crossedFromLeft {
						p.X = m.wallX - p.Radius
					} else {
						p.X = m.wallX + p.Radius
					}
					// Reflect X component and add small angle perturbation (preserving magnitude)
					speed := math.Sqrt(p.VX*p.VX + p.VY*p.VY)
					anglePerturbation := (rand.Float64()*2.0 - 1.0) * 0.05
					// Reflect and perturb angle
					reflectedAngle := math.Atan2(p.VY, -p.VX) + anglePerturbation
					p.VX = speed * math.Cos(reflectedAngle)
					p.VY = speed * math.Sin(reflectedAngle)
				}
				// Otherwise (in door opening), particle passes through
			}
		}
	}
	
	// Check win condition: all particles on left side
	allOnLeft := true
	for _, p := range m.particles {
		if p.X >= m.wallX {
			allOnLeft = false
			break
		}
	}
	
	if allOnLeft && !m.won {
		m.won = true
		m.winTime = time.Now()
	}
	
	return nil
}

// Draw draws the Maxwell's Daemon chapter
func (m *MaxwellsDaemonChapter) Draw(screen *ebiten.Image) {
	// Fill background
	screen.Fill(color.RGBA{10, 10, 20, 255})
	
	// Draw title
	titleText := "Maxwell's Daemon"
	titleBounds := text.BoundString(basicfont.Face7x13, titleText)
	titleX := (screenWidth - titleBounds.Dx()) / 2
	titleY := 20
	text.Draw(screen, titleText, basicfont.Face7x13, titleX, titleY, color.White)
	
	// Draw level and particle count
	infoText := "Level: " + formatNumber(strconv.Itoa(m.level)) + " | Particles: " + formatNumber(strconv.Itoa(m.particleCount))
	infoBounds := text.BoundString(basicfont.Face7x13, infoText)
	infoX := (screenWidth - infoBounds.Dx()) / 2
	infoY := 40
	text.Draw(screen, infoText, basicfont.Face7x13, infoX, infoY, color.RGBA{200, 200, 255, 255})
	
	// Draw box outline
	ebitenutil.DrawRect(screen, m.boxX, m.boxY, m.boxWidth, m.boxHeight, color.RGBA{100, 100, 150, 255})
	
	// Draw wall
	wallColor := color.RGBA{150, 150, 200, 255}
	if m.doorOpen {
		wallColor = color.RGBA{100, 200, 100, 255} // Green when open
	}
	
	// Draw top part of wall (above door)
	wallTopHeight := m.doorCenterY - m.doorHeight/2 - m.wallTopY
	if wallTopHeight > 0 {
		ebitenutil.DrawRect(screen, m.wallX-2, m.wallTopY, 4, wallTopHeight, wallColor)
	}
	
	// Draw bottom part of wall (below door)
	wallBottomStart := m.doorCenterY + m.doorHeight/2
	wallBottomHeight := m.wallBottomY - wallBottomStart
	if wallBottomHeight > 0 {
		ebitenutil.DrawRect(screen, m.wallX-2, wallBottomStart, 4, wallBottomHeight, wallColor)
	}
	
	// Draw door opening indicator
	if m.doorOpen {
		// Draw green highlight for open door
		ebitenutil.DrawRect(screen, m.wallX-3, m.doorCenterY-m.doorHeight/2, 6, m.doorHeight, color.RGBA{50, 200, 50, 100})
	}
	
	// Draw particles
	particleColor := color.RGBA{255, 200, 100, 255}
	for _, p := range m.particles {
		// Draw particle as a circle (using a small rectangle approximation)
		ebitenutil.DrawRect(screen, p.X-p.Radius, p.Y-p.Radius, p.Radius*2, p.Radius*2, particleColor)
	}
	
	// Draw win message
	if m.won {
		winText := "Level Complete! Next level starting..."
		winBounds := text.BoundString(basicfont.Face7x13, winText)
		winX := (screenWidth - winBounds.Dx()) / 2
		winY := int(m.boxY + m.boxHeight + 30)
		text.Draw(screen, winText, basicfont.Face7x13, winX, winY, color.RGBA{100, 255, 100, 255})
	}
	
	// Draw instructions
	instructions := "SPACE: toggle door | Goal: get all particles to the left side"
	instBounds := text.BoundString(basicfont.Face7x13, instructions)
	instX := (screenWidth - instBounds.Dx()) / 2
	instY := screenHeight - 15
	text.Draw(screen, instructions, basicfont.Face7x13, instX, instY, color.Gray{Y: 150})
}

// WasEscConsumed returns whether ESC was consumed by a dialog this frame
func (m *MaxwellsDaemonChapter) WasEscConsumed() bool {
	return m.escConsumed
}

