package main

import (
	"github.com/hajimehoshi/ebiten/v2"
)

// ThreeRowBootlaceChapter implements the Three Row Bootlace chapter
type ThreeRowBootlaceChapter struct {
	escConsumed bool // Whether ESC was consumed by a dialog this frame
}

// NewThreeRowBootlaceChapter creates a new Three Row Bootlace chapter
func NewThreeRowBootlaceChapter() *ThreeRowBootlaceChapter {
	return &ThreeRowBootlaceChapter{}
}

// Update updates the Three Row Bootlace chapter
func (t *ThreeRowBootlaceChapter) Update() error {
	t.escConsumed = false // Reset at start of frame
	// TODO: Implement chapter logic
	return nil
}

// Draw draws the Three Row Bootlace chapter
func (t *ThreeRowBootlaceChapter) Draw(screen *ebiten.Image) {
	// TODO: Implement chapter rendering
	drawTODO(screen, "Three Row Bootlace")
}

// WasEscConsumed returns whether ESC was consumed by a dialog this frame
func (t *ThreeRowBootlaceChapter) WasEscConsumed() bool {
	return t.escConsumed
}

