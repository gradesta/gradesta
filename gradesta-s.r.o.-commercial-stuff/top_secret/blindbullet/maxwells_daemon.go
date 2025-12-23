package main

import (
	"github.com/hajimehoshi/ebiten/v2"
)

// MaxwellsDaemonChapter implements the Maxwell's Daemon chapter
type MaxwellsDaemonChapter struct {
	escConsumed bool // Whether ESC was consumed by a dialog this frame
}

// NewMaxwellsDaemonChapter creates a new Maxwell's Daemon chapter
func NewMaxwellsDaemonChapter() *MaxwellsDaemonChapter {
	return &MaxwellsDaemonChapter{}
}

// Update updates the Maxwell's Daemon chapter
func (m *MaxwellsDaemonChapter) Update() error {
	m.escConsumed = false // Reset at start of frame
	// TODO: Implement chapter logic
	return nil
}

// Draw draws the Maxwell's Daemon chapter
func (m *MaxwellsDaemonChapter) Draw(screen *ebiten.Image) {
	// TODO: Implement chapter rendering
	drawTODO(screen, "Maxwell's Daemon")
}

// WasEscConsumed returns whether ESC was consumed by a dialog this frame
func (m *MaxwellsDaemonChapter) WasEscConsumed() bool {
	return m.escConsumed
}

