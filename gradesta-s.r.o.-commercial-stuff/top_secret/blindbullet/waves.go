package main

import (
	"github.com/hajimehoshi/ebiten/v2"
)

// WavesChapter implements the Waves chapter
type WavesChapter struct {
	escConsumed bool // Whether ESC was consumed by a dialog this frame
}

// NewWavesChapter creates a new Waves chapter
func NewWavesChapter() *WavesChapter {
	return &WavesChapter{}
}

// Update updates the Waves chapter
func (w *WavesChapter) Update() error {
	w.escConsumed = false // Reset at start of frame
	// TODO: Implement chapter logic
	return nil
}

// Draw draws the Waves chapter
func (w *WavesChapter) Draw(screen *ebiten.Image) {
	// TODO: Implement chapter rendering
	drawTODO(screen, "Waves")
}

// WasEscConsumed returns whether ESC was consumed by a dialog this frame
func (w *WavesChapter) WasEscConsumed() bool {
	return w.escConsumed
}

