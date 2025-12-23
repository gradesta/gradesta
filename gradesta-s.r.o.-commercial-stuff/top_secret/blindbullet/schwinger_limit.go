package main

import (
	"github.com/hajimehoshi/ebiten/v2"
)

// SchwingerLimitChapter implements the The Schwinger Limit chapter
type SchwingerLimitChapter struct {
	escConsumed bool // Whether ESC was consumed by a dialog this frame
}

// NewSchwingerLimitChapter creates a new The Schwinger Limit chapter
func NewSchwingerLimitChapter() *SchwingerLimitChapter {
	return &SchwingerLimitChapter{}
}

// Update updates the The Schwinger Limit chapter
func (s *SchwingerLimitChapter) Update() error {
	s.escConsumed = false // Reset at start of frame
	// TODO: Implement chapter logic
	return nil
}

// Draw draws the The Schwinger Limit chapter
func (s *SchwingerLimitChapter) Draw(screen *ebiten.Image) {
	// TODO: Implement chapter rendering
	drawTODO(screen, "The Schwinger Limit")
}

// WasEscConsumed returns whether ESC was consumed by a dialog this frame
func (s *SchwingerLimitChapter) WasEscConsumed() bool {
	return s.escConsumed
}

