package main

import (
	"github.com/hajimehoshi/ebiten/v2"
)

// EssenceOfSelfChapter implements the The Essence of the Self chapter
type EssenceOfSelfChapter struct {
	escConsumed bool // Whether ESC was consumed by a dialog this frame
}

// NewEssenceOfSelfChapter creates a new The Essence of the Self chapter
func NewEssenceOfSelfChapter() *EssenceOfSelfChapter {
	return &EssenceOfSelfChapter{}
}

// Update updates the The Essence of the Self chapter
func (e *EssenceOfSelfChapter) Update() error {
	e.escConsumed = false // Reset at start of frame
	// TODO: Implement chapter logic
	return nil
}

// Draw draws the The Essence of the Self chapter
func (e *EssenceOfSelfChapter) Draw(screen *ebiten.Image) {
	// TODO: Implement chapter rendering
	drawTODO(screen, "The Essence of the Self")
}

// WasEscConsumed returns whether ESC was consumed by a dialog this frame
func (e *EssenceOfSelfChapter) WasEscConsumed() bool {
	return e.escConsumed
}

