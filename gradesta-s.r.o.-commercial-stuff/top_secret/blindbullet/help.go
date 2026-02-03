package main

import (
	"fmt"
	"image/color"
	"strings"

	"github.com/hajimehoshi/ebiten/v2"
	"github.com/hajimehoshi/ebiten/v2/inpututil"
	"github.com/hajimehoshi/ebiten/v2/text"
	"golang.org/x/image/font/basicfont"
)

// HelpDialogState manages the state of a help dialog
type HelpDialogState struct {
	ShowHelp       bool
	HelpScrollOffset int
}

// HandleHelpInput processes input for help dialog (toggle, scroll, close)
// Returns true if help dialog consumed the input (should return early from Update)
func HandleHelpInput(state *HelpDialogState, escConsumed *bool) bool {
	// Handle 'h' key to toggle help dialog
	if inpututil.IsKeyJustPressed(ebiten.KeyH) {
		state.ShowHelp = !state.ShowHelp
		if state.ShowHelp {
			state.HelpScrollOffset = 0
		}
		return true
	}
	
	// Handle help dialog scrolling
	if state.ShowHelp {
		if inpututil.IsKeyJustPressed(ebiten.KeyArrowDown) || inpututil.IsKeyJustPressed(ebiten.KeyS) {
			state.HelpScrollOffset += 15
		}
		if inpututil.IsKeyJustPressed(ebiten.KeyArrowUp) || inpututil.IsKeyJustPressed(ebiten.KeyW) {
			state.HelpScrollOffset -= 15
			if state.HelpScrollOffset < 0 {
				state.HelpScrollOffset = 0
			}
		}
		// Close help with Escape or 'h' again
		if inpututil.IsKeyJustPressed(ebiten.KeyEscape) || inpututil.IsKeyJustPressed(ebiten.KeyH) {
			state.ShowHelp = false
			if escConsumed != nil {
				*escConsumed = true
			}
		}
		// Don't process other keys when help is open
		return true
	}
	
	return false
}

// DrawHelpDialog draws a help dialog with scrollable content
func DrawHelpDialog(screen *ebiten.Image, helpLines []string, scrollOffset int) {
	// Draw semi-transparent overlay
	DrawModalOverlay(screen)
	
	// Draw help dialog box
	dialogWidth := 600.0
	dialogHeight := 500.0
	dialogX := (float64(screenWidth) - dialogWidth) / 2
	dialogY := (float64(screenHeight) - dialogHeight) / 2
	
	DrawModalBox(screen, dialogX, dialogY, dialogWidth, dialogHeight)
	DrawModalTitle(screen, "HELP - KEYBOARD CONTROLS", dialogX, dialogY, dialogWidth)
	
	// Draw scrollable content
	startY := int(dialogY) + 50 - scrollOffset
	lineHeight := 15
	
	for i, line := range helpLines {
		y := startY + i*lineHeight
		// Only draw visible lines
		if y >= int(dialogY)+40 && y <= int(dialogY)+int(dialogHeight)-30 {
			// Color code different sections
			var lineColor color.Color = color.Gray{Y: 200}
			if len(line) > 0 && line[0] != ' ' {
				// Section headers
				lineColor = color.White
			} else if strings.HasPrefix(line, "  ") {
				// Regular lines
				lineColor = color.Gray{Y: 180}
			}
			text.Draw(screen, line, basicfont.Face7x13, int(dialogX+20), y, lineColor)
		}
	}
	
	// Draw scroll indicator if content is scrollable
	totalHeight := len(helpLines) * lineHeight
	if totalHeight > int(dialogHeight-70) {
		// Show scroll position
		scrollText := fmt.Sprintf("Scroll: %d/%d", scrollOffset, totalHeight-int(dialogHeight-70))
		scrollBounds := text.BoundString(basicfont.Face7x13, scrollText)
		scrollX := int(dialogX + dialogWidth - float64(scrollBounds.Dx()) - 10)
		scrollY := int(dialogY + dialogHeight - 20)
		text.Draw(screen, scrollText, basicfont.Face7x13, scrollX, scrollY, color.Gray{Y: 120})
	}
}
