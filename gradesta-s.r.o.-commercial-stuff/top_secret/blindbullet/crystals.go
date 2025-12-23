package main

import (
	"github.com/hajimehoshi/ebiten/v2"
)

// CrystalsChapter implements the Crystals chapter
type CrystalsChapter struct {
	escConsumed bool // Whether ESC was consumed by a dialog this frame
}

// NewCrystalsChapter creates a new Crystals chapter
func NewCrystalsChapter() *CrystalsChapter {
	return &CrystalsChapter{}
}

// Update updates the Crystals chapter
func (c *CrystalsChapter) Update() error {
	c.escConsumed = false // Reset at start of frame
	// TODO: Implement chapter logic
	return nil
}

// Draw draws the Crystals chapter
func (c *CrystalsChapter) Draw(screen *ebiten.Image) {
	// TODO: Implement chapter rendering
	drawTODO(screen, "Crystals")
}

// WasEscConsumed returns whether ESC was consumed by a dialog this frame
func (c *CrystalsChapter) WasEscConsumed() bool {
	return c.escConsumed
}

