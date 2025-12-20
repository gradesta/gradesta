package main

import (
	"image/color"
	"log"

	"github.com/hajimehoshi/ebiten/v2"
	"github.com/hajimehoshi/ebiten/v2/inpututil"
	"github.com/hajimehoshi/ebiten/v2/text"
	"golang.org/x/image/font/basicfont"
)

const (
	screenWidth  = 800
	screenHeight = 600
)

// GameState represents the current state of the game
type GameState int

const (
	StateLaunchScreen GameState = iota
	StateStairs
)

// Game is the main game struct
type Game struct {
	state GameState
	stairs *StairsChapter
}

// NewGame creates a new game instance
func NewGame() *Game {
	return &Game{
		state:  StateLaunchScreen,
		stairs: NewStairsChapter(),
	}
}

func (g *Game) Update() error {
	switch g.state {
	case StateLaunchScreen:
		if inpututil.IsKeyJustPressed(ebiten.KeyEnter) || inpututil.IsKeyJustPressed(ebiten.KeySpace) {
			g.state = StateStairs
		}
	case StateStairs:
		if err := g.stairs.Update(); err != nil {
			return err
		}
		if inpututil.IsKeyJustPressed(ebiten.KeyEscape) {
			g.state = StateLaunchScreen
		}
	}
	return nil
}

func (g *Game) Draw(screen *ebiten.Image) {
	screen.Fill(color.RGBA{20, 20, 30, 255})

	switch g.state {
	case StateLaunchScreen:
		g.drawLaunchScreen(screen)
	case StateStairs:
		g.stairs.Draw(screen)
	}
}

func (g *Game) drawLaunchScreen(screen *ebiten.Image) {
	// Draw title
	titleText := "BLIND BULLET"
	titleBounds := text.BoundString(basicfont.Face7x13, titleText)
	titleX := (screenWidth - titleBounds.Dx()) / 2
	titleY := screenHeight/2 - 50
	text.Draw(screen, titleText, basicfont.Face7x13, titleX, titleY, color.White)

	// Draw instructions
	instructions := "Press ENTER or SPACE to start"
	instBounds := text.BoundString(basicfont.Face7x13, instructions)
	instX := (screenWidth - instBounds.Dx()) / 2
	instY := screenHeight/2 + 50
	text.Draw(screen, instructions, basicfont.Face7x13, instX, instY, color.Gray{Y: 150})

	// Draw chapter list
	chapterText := "Chapters:"
	text.Draw(screen, chapterText, basicfont.Face7x13, 50, screenHeight-100, color.Gray{Y: 200})
	
	stairsText := "- Stairs (Press ENTER)"
	text.Draw(screen, stairsText, basicfont.Face7x13, 50, screenHeight-80, color.Gray{Y: 150})
}

func (g *Game) Layout(outsideWidth, outsideHeight int) (int, int) {
	return screenWidth, screenHeight
}

func main() {
	ebiten.SetWindowSize(screenWidth, screenHeight)
	ebiten.SetWindowTitle("Blind Bullet")
	ebiten.SetWindowResizable(true)

	game := NewGame()
	if err := ebiten.RunGame(game); err != nil {
		log.Fatal(err)
	}
}

