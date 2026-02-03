package main

import (
	"image/color"

	"github.com/hajimehoshi/ebiten/v2"
	"github.com/hajimehoshi/ebiten/v2/ebitenutil"
	"github.com/hajimehoshi/ebiten/v2/text"
	"golang.org/x/image/font/basicfont"
)

// DrawModalOverlay draws a semi-transparent overlay covering the entire screen
func DrawModalOverlay(screen *ebiten.Image) {
	overlayColor := color.RGBA{0, 0, 0, 200}
	ebitenutil.DrawRect(screen, 0, 0, float64(screenWidth), float64(screenHeight), overlayColor)
}

// DrawModalBox draws a modal dialog box with background and border
func DrawModalBox(screen *ebiten.Image, x, y, width, height float64) {
	// Draw dialog background
	ebitenutil.DrawRect(screen, x, y, width, height, color.RGBA{30, 30, 40, 255})
	
	// Draw dialog border
	ebitenutil.DrawRect(screen, x, y, width, 2, color.White)
	ebitenutil.DrawRect(screen, x, y, 2, height, color.White)
	ebitenutil.DrawRect(screen, x+width-2, y, 2, height, color.White)
	ebitenutil.DrawRect(screen, x, y+height-2, width, 2, color.White)
}

// DrawModalTitle draws a centered title in a modal dialog
func DrawModalTitle(screen *ebiten.Image, title string, dialogX, dialogY, dialogWidth float64) {
	titleBounds := text.BoundString(basicfont.Face7x13, title)
	titleX := int(dialogX + (dialogWidth-float64(titleBounds.Dx()))/2)
	titleY := int(dialogY + 20)
	text.Draw(screen, title, basicfont.Face7x13, titleX, titleY, color.White)
}
