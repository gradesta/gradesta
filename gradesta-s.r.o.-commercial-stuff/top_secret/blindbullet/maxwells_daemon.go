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
	IsWall bool    // True if this is a wall particle (level 3+)
}

// Spring represents a spring connection between two particles (for level 3+ walls)
type Spring struct {
	Particle1 int     // Index of first particle
	Particle2 int     // Index of second particle
	RestLength float64 // Rest length of spring
	K         float64 // Spring constant
}

// Photon represents an animated photon emission
type Photon struct {
	X, Y       float64 // Current position (head)
	VX, VY     float64 // Velocity
	Angle      float64 // Direction angle
	Age        float64 // Age in seconds (for animation)
	MaxAge     float64 // Maximum age before disappearing
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
	
	// Level 3+ mechanics
	springs         []Spring // Spring connections for wall particles (level 3+)
	originalTemperature float64 // Tracked for level 3+
	currentTemperature  float64 // Current temperature (updated every 100ms)
	lastTemperatureUpdate time.Time // Last time temperature was updated
	universeCold    bool     // True when universe has cooled
	gravityEnabled  bool     // True when gravity is active
	gravityStartTime time.Time // When gravity was enabled
	springsRemoved  bool     // True when springs are removed
	slowdownPhase   bool     // True when slowing down particles
	photonPhase     bool     // True when showing photon visualization
	photonPhaseStart time.Time // When photon phase started
	photons         []Photon // Active photon emissions
	lastPhotonSpawn time.Time // Last time a photon was spawned
	heatDeathDialog bool     // True when heat death dialog was already shown
	chapterComplete bool     // True when level 3 is complete (after heat death dialog)
	goToNextChapter bool     // True when player chose to go to next chapter
	
	// Dialog state
	showColdDialog  bool     // Show "universe is getting cold" dialog
	showHeatDeathDialog bool // Show heat death dialog
	
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
	rand.Seed(time.Now().UnixNano())
	
	// Reset level 3+ state (but preserve cold phase state if already in it)
	wasInColdPhase := m.universeCold
	if !m.universeCold {
		m.universeCold = false
		m.gravityEnabled = false
		m.springsRemoved = false
		m.photonPhase = false
		m.heatDeathDialog = false
	}
	m.showColdDialog = false
	m.showHeatDeathDialog = false
	
	// Only reset springs if not in cold phase (springs are removed in cold phase)
	if !wasInColdPhase {
		m.springs = nil
	}
	
	if m.level >= 3 {
		// Level 3+: Create wall particles with springs (only if not in cold phase)
		if !wasInColdPhase {
			m.initWallParticles()
		}
	} else {
		// Levels 1-2: Normal static walls
		m.particles = make([]Particle, m.particleCount)
	}
	
	leftCount := m.particleCount / 2
	
	// Initialize game particles on left side
	for i := 0; i < leftCount; i++ {
		particle := Particle{
			X:      m.boxX + 50 + rand.Float64()*(m.wallX-m.boxX-100),
			Y:      m.boxY + 50 + rand.Float64()*(m.boxHeight-100),
			VX:     (rand.Float64()*2 - 1) * m.maxVelocity,
			VY:     (rand.Float64()*2 - 1) * m.maxVelocity,
			Radius: m.particleRadius,
			IsWall: false,
		}
		if m.level >= 3 {
			m.particles = append(m.particles, particle)
		} else {
			m.particles[i] = particle
		}
	}
	
	// Initialize game particles on right side
	for i := leftCount; i < m.particleCount; i++ {
		particle := Particle{
			X:      m.wallX + 50 + rand.Float64()*(m.boxX+m.boxWidth-m.wallX-100),
			Y:      m.boxY + 50 + rand.Float64()*(m.boxHeight-100),
			VX:     (rand.Float64()*2 - 1) * m.maxVelocity,
			VY:     (rand.Float64()*2 - 1) * m.maxVelocity,
			Radius: m.particleRadius,
			IsWall: false,
		}
		if m.level >= 3 {
			m.particles = append(m.particles, particle)
		} else {
			m.particles[i] = particle
		}
	}
	
	// Calculate and store original temperature (for level 3+, only if not already in cold phase)
	if m.level >= 3 && !wasInColdPhase {
		m.originalTemperature = m.calculateTemperature()
		m.currentTemperature = m.originalTemperature
		m.lastTemperatureUpdate = time.Now()
	}
	
	m.won = false
}

// initWallParticles creates wall particles connected by springs for the entire box outline (level 3+)
func (m *MaxwellsDaemonChapter) initWallParticles() {
	springConstant := 0.5
	particleSpacing := 15.0
	wallParticleRadius := 3.0
	
	// Create particles for the entire box outline:
	// 1. Top edge (left to right)
	// 2. Right edge (top to bottom)
	// 3. Bottom edge (right to left)
	// 4. Left edge (bottom to top)
	// 5. Middle dividing wall (top and bottom sections, with door gap)
	
	// Top edge
	topEdgeParticles := int(m.boxWidth / particleSpacing)
	topEdgeStartIdx := len(m.particles)
	for i := 0; i < topEdgeParticles; i++ {
		x := m.boxX + float64(i)*particleSpacing
		m.particles = append(m.particles, Particle{
			X:      x,
			Y:      m.boxY,
			VX:     0,
			VY:     0,
			Radius: wallParticleRadius,
			IsWall: true,
		})
	}
	// Springs for top edge
	for i := 0; i < topEdgeParticles-1; i++ {
		m.springs = append(m.springs, Spring{
			Particle1: topEdgeStartIdx + i,
			Particle2: topEdgeStartIdx + i + 1,
			RestLength: particleSpacing,
			K:         springConstant,
		})
	}
	
	// Right edge
	rightEdgeParticles := int(m.boxHeight / particleSpacing)
	rightEdgeStartIdx := len(m.particles)
	for i := 0; i < rightEdgeParticles; i++ {
		y := m.boxY + float64(i)*particleSpacing
		m.particles = append(m.particles, Particle{
			X:      m.boxX + m.boxWidth,
			Y:      y,
			VX:     0,
			VY:     0,
			Radius: wallParticleRadius,
			IsWall: true,
		})
	}
	// Springs for right edge
	for i := 0; i < rightEdgeParticles-1; i++ {
		m.springs = append(m.springs, Spring{
			Particle1: rightEdgeStartIdx + i,
			Particle2: rightEdgeStartIdx + i + 1,
			RestLength: particleSpacing,
			K:         springConstant,
		})
	}
	// Connect top-right corner
	m.springs = append(m.springs, Spring{
		Particle1: topEdgeStartIdx + topEdgeParticles - 1,
		Particle2: rightEdgeStartIdx,
		RestLength: particleSpacing,
		K:         springConstant,
	})
	
	// Bottom edge
	bottomEdgeParticles := int(m.boxWidth / particleSpacing)
	bottomEdgeStartIdx := len(m.particles)
	for i := 0; i < bottomEdgeParticles; i++ {
		x := m.boxX + m.boxWidth - float64(i)*particleSpacing
		m.particles = append(m.particles, Particle{
			X:      x,
			Y:      m.boxY + m.boxHeight,
			VX:     0,
			VY:     0,
			Radius: wallParticleRadius,
			IsWall: true,
		})
	}
	// Springs for bottom edge
	for i := 0; i < bottomEdgeParticles-1; i++ {
		m.springs = append(m.springs, Spring{
			Particle1: bottomEdgeStartIdx + i,
			Particle2: bottomEdgeStartIdx + i + 1,
			RestLength: particleSpacing,
			K:         springConstant,
		})
	}
	// Connect bottom-right corner
	m.springs = append(m.springs, Spring{
		Particle1: rightEdgeStartIdx + rightEdgeParticles - 1,
		Particle2: bottomEdgeStartIdx,
		RestLength: particleSpacing,
		K:         springConstant,
	})
	
	// Left edge
	leftEdgeParticles := int(m.boxHeight / particleSpacing)
	leftEdgeStartIdx := len(m.particles)
	for i := 0; i < leftEdgeParticles; i++ {
		y := m.boxY + m.boxHeight - float64(i)*particleSpacing
		m.particles = append(m.particles, Particle{
			X:      m.boxX,
			Y:      y,
			VX:     0,
			VY:     0,
			Radius: wallParticleRadius,
			IsWall: true,
		})
	}
	// Springs for left edge
	for i := 0; i < leftEdgeParticles-1; i++ {
		m.springs = append(m.springs, Spring{
			Particle1: leftEdgeStartIdx + i,
			Particle2: leftEdgeStartIdx + i + 1,
			RestLength: particleSpacing,
			K:         springConstant,
		})
	}
	// Connect bottom-left corner
	m.springs = append(m.springs, Spring{
		Particle1: bottomEdgeStartIdx + bottomEdgeParticles - 1,
		Particle2: leftEdgeStartIdx,
		RestLength: particleSpacing,
		K:         springConstant,
	})
	// Connect top-left corner
	m.springs = append(m.springs, Spring{
		Particle1: leftEdgeStartIdx + leftEdgeParticles - 1,
		Particle2: topEdgeStartIdx,
		RestLength: particleSpacing,
		K:         springConstant,
	})
	
	// Middle dividing wall (top and bottom sections, with door gap)
	// Top wall section (above door)
	topWallStart := m.wallTopY
	topWallEnd := m.doorCenterY - m.doorHeight/2
	topWallParticles := int((topWallEnd - topWallStart) / particleSpacing)
	
	topWallStartIdx := len(m.particles)
	for i := 0; i < topWallParticles; i++ {
		y := topWallStart + float64(i)*particleSpacing
		m.particles = append(m.particles, Particle{
			X:      m.wallX,
			Y:      y,
			VX:     0,
			VY:     0,
			Radius: wallParticleRadius,
			IsWall: true,
		})
	}
	// Create springs for top wall
	for i := 0; i < topWallParticles-1; i++ {
		m.springs = append(m.springs, Spring{
			Particle1: topWallStartIdx + i,
			Particle2: topWallStartIdx + i + 1,
			RestLength: particleSpacing,
			K:         springConstant,
		})
	}
	
	// Bottom wall section (below door)
	bottomWallStart := m.doorCenterY + m.doorHeight/2
	bottomWallEnd := m.wallBottomY
	bottomWallParticles := int((bottomWallEnd - bottomWallStart) / particleSpacing)
	
	bottomWallStartIdx := len(m.particles)
	for i := 0; i < bottomWallParticles; i++ {
		y := bottomWallStart + float64(i)*particleSpacing
		m.particles = append(m.particles, Particle{
			X:      m.wallX,
			Y:      y,
			VX:     0,
			VY:     0,
			Radius: wallParticleRadius,
			IsWall: true,
		})
	}
	// Create springs for bottom wall
	for i := 0; i < bottomWallParticles-1; i++ {
		m.springs = append(m.springs, Spring{
			Particle1: bottomWallStartIdx + i,
			Particle2: bottomWallStartIdx + i + 1,
			RestLength: particleSpacing,
			K:         springConstant,
		})
	}
}

// calculateTemperature calculates the temperature based on particles inside the box and their velocities
// Temperature is proportional to the average kinetic energy (velocity squared) of particles inside the box
func (m *MaxwellsDaemonChapter) calculateTemperature() float64 {
	particlesInsideBox := 0
	totalKineticEnergy := 0.0
	
	// Count particles inside the box and sum their kinetic energy
	for _, p := range m.particles {
		if !p.IsWall {
			// Check if particle is inside the box boundaries
			if p.X >= m.boxX && p.X <= m.boxX+m.boxWidth &&
			   p.Y >= m.boxY && p.Y <= m.boxY+m.boxHeight {
				particlesInsideBox++
				// Kinetic energy is proportional to velocity squared
				speedSquared := p.VX*p.VX + p.VY*p.VY
				totalKineticEnergy += speedSquared
			}
		}
	}
	
	if particlesInsideBox == 0 {
		return 0
	}
	
	// Temperature is average kinetic energy per particle
	return totalKineticEnergy / float64(particlesInsideBox)
}

// Update updates the Maxwell's Daemon chapter
func (m *MaxwellsDaemonChapter) Update() error {
	m.escConsumed = false // Reset at start of frame
	
	// Handle dialogs
	if m.showColdDialog {
		if inpututil.IsKeyJustPressed(ebiten.KeyEnter) || inpututil.IsKeyJustPressed(ebiten.KeySpace) {
			m.showColdDialog = false
			m.universeCold = true
			m.gravityEnabled = true
			m.gravityStartTime = time.Now()
			m.springsRemoved = true
			// Actually remove the springs from the array (but keep particles intact)
			m.springs = nil
			// Reset win state to prevent level transition
			m.won = false
			// Keep IsWall flags as-is - particles remain in place with their velocities
			// The rendering will continue to show them as wall particles
		}
		return nil
	}
	
	if m.showHeatDeathDialog {
		if inpututil.IsKeyJustPressed(ebiten.KeyS) {
			// Stay and continue to level 4 (normal gameplay)
			m.showHeatDeathDialog = false
			m.level = 4
			m.particleCount = 48 // Reset to reasonable count for level 4
			m.universeCold = false
			m.gravityEnabled = false
			m.gravityStartTime = time.Time{} // Reset timer
			m.springsRemoved = false
			m.slowdownPhase = false
			m.photonPhase = false
			m.photonPhaseStart = time.Time{}
			m.photons = nil
			m.heatDeathDialog = false
			m.chapterComplete = false
			m.initParticles()
		} else if inpututil.IsKeyJustPressed(ebiten.KeyEnter) || inpututil.IsKeyJustPressed(ebiten.KeySpace) {
			// Go to next chapter immediately
			m.showHeatDeathDialog = false
			m.goToNextChapter = true
		}
		return nil
	}
	
	// Handle spacebar to toggle door (only if not in cold phase)
	if !m.universeCold && inpututil.IsKeyJustPressed(ebiten.KeySpace) {
		m.doorOpen = !m.doorOpen
	}
	
	// Cheat: Ctrl+S to skip level (not documented)
	if ebiten.IsKeyPressed(ebiten.KeyControl) && inpututil.IsKeyJustPressed(ebiten.KeyS) {
		m.level++
		m.particleCount *= 2
		m.initParticles()
		return nil
	}
	
	// If won, wait a bit then advance to next level (only for levels 1-2, not in cold phase)
	if m.won && !m.universeCold {
		if time.Since(m.winTime) > 2*time.Second {
			m.level++
			m.particleCount *= 2
			m.initParticles()
		}
		return nil
	}
	
	// Level 3+: Update spring physics for wall particles (before removing springs)
	if m.level >= 3 && !m.springsRemoved {
		m.updateSprings()
	}
	
	// Level 3+: Update temperature every 100ms
	if m.level >= 3 {
		now := time.Now()
		if now.Sub(m.lastTemperatureUpdate) >= 100*time.Millisecond {
			m.currentTemperature = m.calculateTemperature()
			m.lastTemperatureUpdate = now
			
			// Check if universe has cooled enough to show dialog
			if !m.universeCold && m.originalTemperature > 0 {
				if m.currentTemperature < m.originalTemperature*0.50 {
					m.showColdDialog = true
				}
			}
		}
	}
	
	// Level 3+: After 5 seconds of gravity, start slowdown and photon phase
	if m.gravityEnabled && !m.slowdownPhase && !m.gravityStartTime.IsZero() {
		if time.Since(m.gravityStartTime) > 5*time.Second {
			m.slowdownPhase = true
			m.photonPhase = true
			m.photonPhaseStart = time.Now()
		}
	}
	
	// Level 3+: During slowdown phase, rapidly slow down all particles and spawn photons
	if m.slowdownPhase {
		photonPhaseDuration := time.Since(m.photonPhaseStart).Seconds()
		
		// Apply damping to slow down particles
		for i := range m.particles {
			p := &m.particles[i]
			p.VX *= 0.95
			p.VY *= 0.95
		}
		
		// Find the densest cluster of particles (center of the largest blob)
		blobCenterX, blobCenterY := m.findDensestClusterCenter()
		
		// Calculate spawn rate that decreases over time
		// Start at 100ms intervals, increase to 500ms over 3 seconds
		spawnInterval := 100.0 + photonPhaseDuration*133.0 // ms
		if spawnInterval > 500.0 {
			spawnInterval = 500.0
		}
		
		// Spawn new photons periodically (fewer over time)
		if time.Since(m.lastPhotonSpawn) > time.Duration(spawnInterval)*time.Millisecond {
			m.lastPhotonSpawn = time.Now()
			// Spawn fewer photons over time (start with 2-3, decrease to 1)
			numToSpawn := 3 - int(photonPhaseDuration/1.5)
			if numToSpawn < 1 {
				numToSpawn = 1
			}
			for j := 0; j < numToSpawn; j++ {
				angle := rand.Float64() * 2.0 * math.Pi
				speed := 3.0 + rand.Float64()*2.0 // Random speed
				photon := Photon{
					X:      blobCenterX,
					Y:      blobCenterY,
					VX:     math.Cos(angle) * speed,
					VY:     math.Sin(angle) * speed,
					Angle:  angle,
					Age:    0,
					MaxAge: 2.0 + rand.Float64(), // 2-3 seconds before disappearing
				}
				m.photons = append(m.photons, photon)
			}
			
			// Remove a random particle when spawning photons (particles evaporate)
			if len(m.particles) > 1 {
				removeIdx := rand.Intn(len(m.particles))
				m.particles = append(m.particles[:removeIdx], m.particles[removeIdx+1:]...)
			}
		}
		
		// Update photon positions and ages
		dt := 1.0 / 60.0 // Approximate frame time
		newPhotons := make([]Photon, 0, len(m.photons))
		for i := range m.photons {
			p := &m.photons[i]
			p.X += p.VX
			p.Y += p.VY
			p.Age += dt
			// Keep photon if not expired
			if p.Age < p.MaxAge {
				newPhotons = append(newPhotons, *p)
			}
		}
		m.photons = newPhotons
		
		// Show heat death dialog after 3 seconds of photon phase
		if photonPhaseDuration > 3.0 && !m.heatDeathDialog {
			m.showHeatDeathDialog = true
			m.heatDeathDialog = true
		}
	}
	
	// Level 3+: Apply particle-to-particle gravity if enabled
	gravityStrength := 50.0 // Gravity constant for inverse square law between particles
	
	// Apply gravitational attraction between all particles
	if m.gravityEnabled {
		for i := range m.particles {
			p := &m.particles[i]
			// Only apply to particles that should be affected (all particles after springs removed)
			if !p.IsWall || m.springsRemoved {
				for j := i + 1; j < len(m.particles); j++ {
					other := &m.particles[j]
					if !other.IsWall || m.springsRemoved {
						dx := other.X - p.X
						dy := other.Y - p.Y
						distSq := dx*dx + dy*dy
						dist := math.Sqrt(distSq)
						if dist > 5 {
							// Apply inverse square law gravity between particles
							force := gravityStrength / distSq
							// Cap the force to prevent extreme acceleration when very close
							if force > 0.5 {
								force = 0.5
							}
							// Apply equal and opposite forces
							fx := (dx / dist) * force
							fy := (dy / dist) * force
							p.VX += fx
							p.VY += fy
							other.VX -= fx
							other.VY -= fy
						}
					}
				}
			}
		}
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
				
				// In gravity mode (springs removed), particles stick together (inelastic collision)
				if m.springsRemoved {
					// Average velocities - perfectly inelastic collision
					avgVX := (p.VX + other.VX) / 2
					avgVY := (p.VY + other.VY) / 2
					p.VX = avgVX
					p.VY = avgVY
					other.VX = avgVX
					other.VY = avgVY
				} else {
					// Normal bouncing collision (elastic)
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
		}
		
		// Update position
		p.X += p.VX
		p.Y += p.VY
		
		// Collision with walls
		// For level 3+ with springs, check collision with wall particles (box outline only)
		// For all levels, use static wall collision for the middle wall (door) - it has continuous detection
		if m.level >= 3 && !m.springsRemoved {
			// Level 3+: Check collision with BOX OUTLINE wall particles only (skip dividing wall)
			for j := range m.particles {
				wallP := &m.particles[j]
				if wallP.IsWall {
					// Skip dividing wall particles - we handle them with static collision below
					isDividingWall := math.Abs(wallP.X - m.wallX) < 5.0
					if isDividingWall {
						continue
					}
					
					// Box outline wall particle - always bounce
					dx := p.X - wallP.X
					dy := p.Y - wallP.Y
					distance := math.Sqrt(dx*dx + dy*dy)
					minDistance := p.Radius + wallP.Radius
					
					if distance < minDistance && distance > 0 {
						// Collision with wall particle
						normalX := dx / distance
						normalY := dy / distance
						
						// Separate particles
						overlap := minDistance - distance
						p.X += normalX * overlap * 0.5
						p.Y += normalY * overlap * 0.5
						wallP.X -= normalX * overlap * 0.5
						wallP.Y -= normalY * overlap * 0.5
						
						// Bounce game particle off wall particle
						speed := math.Sqrt(p.VX*p.VX + p.VY*p.VY)
						anglePerturbation := (rand.Float64()*2.0 - 1.0) * 0.05
						// Reflect velocity
						dotProduct := p.VX*normalX + p.VY*normalY
						p.VX -= 2 * dotProduct * normalX
						p.VY -= 2 * dotProduct * normalY
						// Apply angle perturbation
						if speed > 0 {
							currentAngle := math.Atan2(p.VY, p.VX)
							newAngle := currentAngle + anglePerturbation
							p.VX = speed * math.Cos(newAngle)
							p.VY = speed * math.Sin(newAngle)
						}
					}
				}
			}
			
			// Level 3+ with springs: Still use static collision for middle wall (door) - has continuous detection
			doorTop := m.doorCenterY - m.doorHeight/2
			doorBottom := m.doorCenterY + m.doorHeight/2
			
			prevLeft := prevX - p.Radius
			prevRight := prevX + p.Radius
			currLeft := p.X - p.Radius
			currRight := p.X + p.Radius
			
			crossedFromLeft := prevRight <= m.wallX && currRight > m.wallX
			crossedFromRight := prevLeft >= m.wallX && currLeft < m.wallX
			
			if crossedFromLeft || crossedFromRight {
				var intersectionY float64
				if crossedFromLeft {
					t := (m.wallX - prevRight) / (currRight - prevRight)
					intersectionY = prevY + t*(p.Y-prevY)
				} else {
					t := (m.wallX - prevLeft) / (currLeft - prevLeft)
					intersectionY = prevY + t*(p.Y-prevY)
				}
				
				inDoorOpening := intersectionY >= doorTop && intersectionY <= doorBottom
				
				if m.doorOpen {
					// Green bar visible = door is CLOSED - always bounce
					if crossedFromLeft {
						p.X = m.wallX - p.Radius
					} else {
						p.X = m.wallX + p.Radius
					}
					speed := math.Sqrt(p.VX*p.VX + p.VY*p.VY)
					anglePerturbation := (rand.Float64()*2.0 - 1.0) * 0.05
					reflectedAngle := math.Atan2(p.VY, -p.VX) + anglePerturbation
					p.VX = speed * math.Cos(reflectedAngle)
					p.VY = speed * math.Sin(reflectedAngle)
				} else {
					// No green bar = door is OPEN
					if !inDoorOpening {
						if crossedFromLeft {
							p.X = m.wallX - p.Radius
						} else {
							p.X = m.wallX + p.Radius
						}
						speed := math.Sqrt(p.VX*p.VX + p.VY*p.VY)
						anglePerturbation := (rand.Float64()*2.0 - 1.0) * 0.05
						reflectedAngle := math.Atan2(p.VY, -p.VX) + anglePerturbation
						p.VX = speed * math.Cos(reflectedAngle)
						p.VY = speed * math.Sin(reflectedAngle)
					}
				}
			}
		} else if m.level < 3 || !m.springsRemoved {
			// Levels 1-2: static wall collision
			// (Level 3+ after springs removed: no wall collision - particles float freely)
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
			// Levels 1-2 or level 3+ after springs removed: static wall collision
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
	}
	
	// Check win condition: all particles on left side (only for levels 1-2, or level 3+ before cold phase)
	if !m.universeCold {
		allOnLeft := true
		for _, p := range m.particles {
			if !p.IsWall && p.X >= m.wallX {
				allOnLeft = false
				break
			}
		}
		
		if allOnLeft && !m.won {
			m.won = true
			m.winTime = time.Now()
		}
	}
	
	return nil
}

// updateSprings updates spring physics for wall particles (level 3+)
func (m *MaxwellsDaemonChapter) updateSprings() {
	if m.springsRemoved {
		return
	}
	
	// Apply spring forces
	for _, spring := range m.springs {
		p1 := &m.particles[spring.Particle1]
		p2 := &m.particles[spring.Particle2]
		
		// Calculate current distance
		dx := p2.X - p1.X
		dy := p2.Y - p1.Y
		distance := math.Sqrt(dx*dx + dy*dy)
		
		if distance > 0 {
			// Calculate spring force (F = -k * (x - restLength))
			displacement := distance - spring.RestLength
			force := spring.K * displacement
			
			// Normalize direction
			normalX := dx / distance
			normalY := dy / distance
			
			// Apply force to both particles (equal and opposite)
			// Assuming equal mass
			p1.VX += normalX * force * 0.5
			p1.VY += normalY * force * 0.5
			p2.VX -= normalX * force * 0.5
			p2.VY -= normalY * force * 0.5
		}
	}
	
	// Apply damping to wall particles to prevent oscillation
	damping := 0.95
	for i := range m.particles {
		if m.particles[i].IsWall {
			m.particles[i].VX *= damping
			m.particles[i].VY *= damping
		}
	}
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
	
	// Draw temperature (for level 3+) - moved here to be visible
	if m.level >= 3 {
		var tempText string
		if m.originalTemperature > 0 {
			tempText = "Temperature: " + strconv.FormatFloat(m.currentTemperature, 'f', 2, 64) + " (Original: " + strconv.FormatFloat(m.originalTemperature, 'f', 2, 64) + ")"
		} else {
			tempText = "Temperature: " + strconv.FormatFloat(m.currentTemperature, 'f', 2, 64)
		}
		tempBounds := text.BoundString(basicfont.Face7x13, tempText)
		tempX := (screenWidth - tempBounds.Dx()) / 2
		tempY := 80
		text.Draw(screen, tempText, basicfont.Face7x13, tempX, tempY, color.RGBA{255, 200, 100, 255})
	}
	
	// Draw wall (level 3+ uses particles for entire box outline, levels 1-2 use static wall)
	if m.level >= 3 {
		// Level 3+: Draw wall particles (and springs if not removed)
		wallColor := color.RGBA{150, 150, 200, 255}
		doorColor := color.RGBA{100, 200, 100, 255} // Green when door is open (closed)
		
		// Draw springs connecting wall particles (only if springs exist)
		if !m.springsRemoved {
			for _, spring := range m.springs {
				p1 := m.particles[spring.Particle1]
				p2 := m.particles[spring.Particle2]
				// Check if this spring is part of the dividing wall (not box outline)
				// Dividing wall particles are at X = m.wallX
				isDividingWall := (p1.X == m.wallX || p2.X == m.wallX) && p1.X == p2.X
				springColor := wallColor
				if isDividingWall && m.doorOpen {
					// Check if spring is in door opening (skip drawing if so)
					doorTop := m.doorCenterY - m.doorHeight/2
					doorBottom := m.doorCenterY + m.doorHeight/2
					avgY := (p1.Y + p2.Y) / 2
					if avgY >= doorTop && avgY <= doorBottom {
						continue // Skip drawing spring in door opening
					}
					springColor = doorColor
				}
				ebitenutil.DrawLine(screen, p1.X, p1.Y, p2.X, p2.Y, springColor)
			}
		}
		
		// Draw wall particles (always, even after springs are removed)
		for _, p := range m.particles {
			if p.IsWall {
				ebitenutil.DrawRect(screen, p.X-p.Radius, p.Y-p.Radius, p.Radius*2, p.Radius*2, wallColor)
			}
		}
		
		// Draw door opening indicator (only if springs not removed)
		if !m.springsRemoved && m.doorOpen {
			ebitenutil.DrawRect(screen, m.wallX-3, m.doorCenterY-m.doorHeight/2, 6, m.doorHeight, color.RGBA{50, 200, 50, 100})
		}
	} else {
		// Levels 1-2 or level 3+ after springs removed: static wall
		// Draw box outline
		ebitenutil.DrawRect(screen, m.boxX, m.boxY, m.boxWidth, m.boxHeight, color.RGBA{100, 100, 150, 255})
		
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
	}
	
	// Draw game particles (non-wall)
	particleColor := color.RGBA{255, 200, 100, 255}
	for _, p := range m.particles {
		if !p.IsWall {
			// Draw particle as a circle (using a small rectangle approximation)
			ebitenutil.DrawRect(screen, p.X-p.Radius, p.Y-p.Radius, p.Radius*2, p.Radius*2, particleColor)
		}
	}
	
	// Level 3+: Draw photon visualization
	if m.photonPhase {
		m.drawPhotons(screen)
	}
	
	// Draw win message
	if m.won {
		winText := "Level Complete! Next level starting..."
		winBounds := text.BoundString(basicfont.Face7x13, winText)
		winX := (screenWidth - winBounds.Dx()) / 2
		winY := int(m.boxY + m.boxHeight + 30)
		text.Draw(screen, winText, basicfont.Face7x13, winX, winY, color.RGBA{100, 255, 100, 255})
	}
	
	// Draw score (particles on left vs right) - draw after everything so it's on top
	leftCount := 0
	rightCount := 0
	for _, p := range m.particles {
		if !p.IsWall {
			if p.X < m.wallX {
				leftCount++
			} else {
				rightCount++
			}
		}
	}
	scoreText := "Left: " + strconv.Itoa(leftCount) + " | Right: " + strconv.Itoa(rightCount)
	text.Draw(screen, scoreText, basicfont.Face7x13, 10, 25, color.RGBA{150, 255, 150, 255})
	
	// Draw instructions (only if not in cold phase)
	if !m.universeCold {
		instructions := "SPACE: toggle door | Goal: get all particles to the left side"
		instBounds := text.BoundString(basicfont.Face7x13, instructions)
		instX := (screenWidth - instBounds.Dx()) / 2
		instY := screenHeight - 15
		text.Draw(screen, instructions, basicfont.Face7x13, instX, instY, color.Gray{Y: 150})
	}
	
	// Draw dialogs
	if m.showColdDialog {
		m.drawColdDialog(screen)
	}
	if m.showHeatDeathDialog {
		m.drawHeatDeathDialog(screen)
	}
}

// calculateCenterOfGravity calculates the center of gravity of all particles
func (m *MaxwellsDaemonChapter) calculateCenterOfGravity() (float64, float64) {
	totalX := 0.0
	totalY := 0.0
	count := 0
	for _, p := range m.particles {
		totalX += p.X
		totalY += p.Y
		count++
	}
	if count == 0 {
		return m.boxX + m.boxWidth/2, m.boxY + m.boxHeight/2
	}
	return totalX / float64(count), totalY / float64(count)
}

// findDensestClusterCenter finds the center of the densest cluster of particles
func (m *MaxwellsDaemonChapter) findDensestClusterCenter() (float64, float64) {
	if len(m.particles) == 0 {
		return m.boxX + m.boxWidth/2, m.boxY + m.boxHeight/2
	}
	
	// For each particle, count how many neighbors are within a radius
	neighborRadius := 50.0
	bestX, bestY := m.particles[0].X, m.particles[0].Y
	bestCount := 0
	
	for i := range m.particles {
		p := &m.particles[i]
		neighborCount := 0
		sumX, sumY := 0.0, 0.0
		
		for j := range m.particles {
			other := &m.particles[j]
			dx := p.X - other.X
			dy := p.Y - other.Y
			dist := math.Sqrt(dx*dx + dy*dy)
			if dist < neighborRadius {
				neighborCount++
				sumX += other.X
				sumY += other.Y
			}
		}
		
		if neighborCount > bestCount {
			bestCount = neighborCount
			// Use the centroid of the cluster, not just the particle position
			bestX = sumX / float64(neighborCount)
			bestY = sumY / float64(neighborCount)
		}
	}
	
	return bestX, bestY
}

// drawPhotons draws the photon visualization (sperm-like sine wave photons)
func (m *MaxwellsDaemonChapter) drawPhotons(screen *ebiten.Image) {
	photonColor := color.RGBA{200, 200, 255, 255}
	tailColor := color.RGBA{150, 150, 220, 200}
	
	for _, photon := range m.photons {
		// Calculate fade based on age
		fadeProgress := photon.Age / photon.MaxAge
		alpha := uint8(255 * (1.0 - fadeProgress))
		headColor := color.RGBA{photonColor.R, photonColor.G, photonColor.B, alpha}
		tailAlpha := uint8(float64(tailColor.A) * (1.0 - fadeProgress))
		fadedTailColor := color.RGBA{tailColor.R, tailColor.G, tailColor.B, tailAlpha}
		
		// Draw the head (larger circle)
		headSize := 4.0
		ebitenutil.DrawRect(screen, photon.X-headSize/2, photon.Y-headSize/2, headSize, headSize, headColor)
		
		// Draw the sine wave tail behind the photon
		tailLength := 40.0
		amplitude := 5.0
		waveFrequency := 3.0 // Number of waves in the tail
		numPoints := 25
		
		// Direction the photon is traveling (opposite for tail)
		tailAngle := photon.Angle + math.Pi
		
		// Perpendicular direction for sine wave
		perpX := -math.Sin(photon.Angle)
		perpY := math.Cos(photon.Angle)
		
		// Animation phase based on age (makes the wave appear to move)
		animationPhase := photon.Age * 15.0
		
		var prevX, prevY float64
		for j := 0; j < numPoints; j++ {
			t := float64(j) / float64(numPoints-1)
			distance := t * tailLength
			
			// Base position along tail direction
			baseX := photon.X + math.Cos(tailAngle)*distance
			baseY := photon.Y + math.Sin(tailAngle)*distance
			
			// Sine wave offset (animated)
			waveOffset := amplitude * (1.0 - t) * math.Sin(t*waveFrequency*2.0*math.Pi + animationPhase)
			
			// Final position
			x := baseX + perpX*waveOffset
			y := baseY + perpY*waveOffset
			
			// Draw line segment from previous point
			if j > 0 {
				ebitenutil.DrawLine(screen, prevX, prevY, x, y, fadedTailColor)
			}
			
			prevX = x
			prevY = y
		}
	}
}

// drawColdDialog draws the "universe is getting cold" dialog
func (m *MaxwellsDaemonChapter) drawColdDialog(screen *ebiten.Image) {
	// Draw semi-transparent overlay
	overlayColor := color.RGBA{0, 0, 0, 200}
	ebitenutil.DrawRect(screen, 0, 0, float64(screenWidth), float64(screenHeight), overlayColor)
	
	// Draw dialog box
	dialogWidth := 500.0
	dialogHeight := 150.0
	dialogX := (float64(screenWidth) - dialogWidth) / 2
	dialogY := (float64(screenHeight) - dialogHeight) / 2
	
	// Draw dialog background
	ebitenutil.DrawRect(screen, dialogX, dialogY, dialogWidth, dialogHeight, color.RGBA{40, 40, 50, 255})
	
	// Draw dialog border
	ebitenutil.DrawRect(screen, dialogX, dialogY, dialogWidth, 2, color.White)
	ebitenutil.DrawRect(screen, dialogX, dialogY, 2, dialogHeight, color.White)
	ebitenutil.DrawRect(screen, dialogX+dialogWidth-2, dialogY, 2, dialogHeight, color.White)
	ebitenutil.DrawRect(screen, dialogX, dialogY+dialogHeight-2, dialogWidth, 2, color.White)
	
	// Draw message
	messageText := "Your universe is getting cold, your universe is dying"
	messageBounds := text.BoundString(basicfont.Face7x13, messageText)
	messageX := int(dialogX + (dialogWidth-float64(messageBounds.Dx()))/2)
	messageY := int(dialogY + 40)
	text.Draw(screen, messageText, basicfont.Face7x13, messageX, messageY, color.White)
	
	// Draw instruction
	instText := "Press ENTER or SPACE to continue"
	instBounds := text.BoundString(basicfont.Face7x13, instText)
	instX := int(dialogX + (dialogWidth-float64(instBounds.Dx()))/2)
	instY := int(dialogY + 80)
	text.Draw(screen, instText, basicfont.Face7x13, instX, instY, color.Gray{Y: 150})
}

// drawHeatDeathDialog draws the heat death dialog
func (m *MaxwellsDaemonChapter) drawHeatDeathDialog(screen *ebiten.Image) {
	// Draw semi-transparent overlay
	overlayColor := color.RGBA{0, 0, 0, 200}
	ebitenutil.DrawRect(screen, 0, 0, float64(screenWidth), float64(screenHeight), overlayColor)
	
	// Draw dialog box
	dialogWidth := 600.0
	dialogHeight := 220.0
	dialogX := (float64(screenWidth) - dialogWidth) / 2
	dialogY := (float64(screenHeight) - dialogHeight) / 2
	
	// Draw dialog background
	ebitenutil.DrawRect(screen, dialogX, dialogY, dialogWidth, dialogHeight, color.RGBA{40, 40, 50, 255})
	
	// Draw dialog border
	ebitenutil.DrawRect(screen, dialogX, dialogY, dialogWidth, 2, color.White)
	ebitenutil.DrawRect(screen, dialogX, dialogY, 2, dialogHeight, color.White)
	ebitenutil.DrawRect(screen, dialogX+dialogWidth-2, dialogY, 2, dialogHeight, color.White)
	ebitenutil.DrawRect(screen, dialogX, dialogY+dialogHeight-2, dialogWidth, 2, color.White)
	
	// Draw message (split into lines)
	messageLine1 := "Your universe is approaching its heat death."
	messageLine2 := "Eventually all matter will evaporate or become static."
	messageLine3 := "Usefull energy will be exhausted."
	messageLine4 := "All that will remain is television static."
	
	line1Bounds := text.BoundString(basicfont.Face7x13, messageLine1)
	line1X := int(dialogX + (dialogWidth-float64(line1Bounds.Dx()))/2)
	line1Y := int(dialogY + 40)
	text.Draw(screen, messageLine1, basicfont.Face7x13, line1X, line1Y, color.White)
	
	line2Bounds := text.BoundString(basicfont.Face7x13, messageLine2)
	line2X := int(dialogX + (dialogWidth-float64(line2Bounds.Dx()))/2)
	line2Y := int(dialogY + 60)
	text.Draw(screen, messageLine2, basicfont.Face7x13, line2X, line2Y, color.White)
	
	line3Bounds := text.BoundString(basicfont.Face7x13, messageLine3)
	line3X := int(dialogX + (dialogWidth-float64(line3Bounds.Dx()))/2)
	line3Y := int(dialogY + 80)
	text.Draw(screen, messageLine3, basicfont.Face7x13, line3X, line3Y, color.White)
	
	line4Bounds := text.BoundString(basicfont.Face7x13, messageLine4)
	line4X := int(dialogX + (dialogWidth-float64(line4Bounds.Dx()))/2)
	line4Y := int(dialogY + 100)
	text.Draw(screen, messageLine4, basicfont.Face7x13, line4X, line4Y, color.White)
	
	// Draw instructions
	instText1 := "Press S to stay and continue to Level 4"
	inst1Bounds := text.BoundString(basicfont.Face7x13, instText1)
	inst1X := int(dialogX + (dialogWidth-float64(inst1Bounds.Dx()))/2)
	inst1Y := int(dialogY + 150)
	text.Draw(screen, instText1, basicfont.Face7x13, inst1X, inst1Y, color.Gray{Y: 150})
	
	instText2 := "Press ENTER or SPACE to go to next chapter"
	inst2Bounds := text.BoundString(basicfont.Face7x13, instText2)
	inst2X := int(dialogX + (dialogWidth-float64(inst2Bounds.Dx()))/2)
	inst2Y := int(dialogY + 180)
	text.Draw(screen, instText2, basicfont.Face7x13, inst2X, inst2Y, color.RGBA{100, 200, 100, 255})
}

// WasEscConsumed returns whether ESC was consumed by a dialog this frame
func (m *MaxwellsDaemonChapter) WasEscConsumed() bool {
	return m.escConsumed
}

// IsChapterComplete returns whether the chapter is complete and should return to menu
func (m *MaxwellsDaemonChapter) IsChapterComplete() bool {
	return m.chapterComplete
}

// ShouldGoToNextChapter returns whether we should go to the next chapter
func (m *MaxwellsDaemonChapter) ShouldGoToNextChapter() bool {
	return m.goToNextChapter
}

