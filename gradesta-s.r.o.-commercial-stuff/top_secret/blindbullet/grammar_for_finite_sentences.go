package main

import (
	"github.com/hajimehoshi/ebiten/v2"
)

// GrammarForFiniteSentencesChapter implements the A Grammar for Finite Sentences chapter
type GrammarForFiniteSentencesChapter struct {
	escConsumed bool // Whether ESC was consumed by a dialog this frame
}

// NewGrammarForFiniteSentencesChapter creates a new A Grammar for Finite Sentences chapter
func NewGrammarForFiniteSentencesChapter() *GrammarForFiniteSentencesChapter {
	return &GrammarForFiniteSentencesChapter{}
}

// Update updates the A Grammar for Finite Sentences chapter
func (g *GrammarForFiniteSentencesChapter) Update() error {
	g.escConsumed = false // Reset at start of frame
	// TODO: Implement chapter logic
	return nil
}

// Draw draws the A Grammar for Finite Sentences chapter
func (g *GrammarForFiniteSentencesChapter) Draw(screen *ebiten.Image) {
	// TODO: Implement chapter rendering
	drawTODO(screen, "A Grammar for Finite Sentences")
}

// WasEscConsumed returns whether ESC was consumed by a dialog this frame
func (g *GrammarForFiniteSentencesChapter) WasEscConsumed() bool {
	return g.escConsumed
}

