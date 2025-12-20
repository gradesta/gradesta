package main

import (
	"errors"
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
	StateExit
)

// Chapter represents a selectable chapter
type Chapter int

const (
	ChapterStairs Chapter = iota
	ChapterExit
	ChapterCount // Total number of chapters
)

// Game is the main game struct
type Game struct {
	state         GameState
	stairs        *StairsChapter
	selectedChapter Chapter
}

// NewGame creates a new game instance
func NewGame() *Game {
	return &Game{
		state:           StateLaunchScreen,
		stairs:          NewStairsChapter(),
		selectedChapter: ChapterStairs,
	}
}

func (g *Game) Update() error {
	switch g.state {
	case StateLaunchScreen:
		// Handle chapter selection with arrow keys
		if inpututil.IsKeyJustPressed(ebiten.KeyArrowUp) {
			g.selectedChapter--
			if g.selectedChapter < 0 {
				g.selectedChapter = ChapterCount - 1
			}
		}
		if inpututil.IsKeyJustPressed(ebiten.KeyArrowDown) {
			g.selectedChapter++
			if g.selectedChapter >= ChapterCount {
				g.selectedChapter = 0
			}
		}
		
		// Activate selected chapter
		if inpututil.IsKeyJustPressed(ebiten.KeyEnter) || inpututil.IsKeyJustPressed(ebiten.KeySpace) {
			switch g.selectedChapter {
			case ChapterStairs:
				g.state = StateStairs
			case ChapterExit:
				return errors.New("user requested exit")
			}
		}
		
		// Allow Escape or Q to exit the application
		if inpututil.IsKeyJustPressed(ebiten.KeyEscape) || inpututil.IsKeyJustPressed(ebiten.KeyQ) {
			return errors.New("user requested exit")
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
	
	// Draw exit instruction
	exitText := "Press ESC or Q to exit"
	exitBounds := text.BoundString(basicfont.Face7x13, exitText)
	exitX := (screenWidth - exitBounds.Dx()) / 2
	exitY := screenHeight/2 + 70
	text.Draw(screen, exitText, basicfont.Face7x13, exitX, exitY, color.Gray{Y: 100})

	// Draw chapter list
	chapterText := "Chapters:"
	text.Draw(screen, chapterText, basicfont.Face7x13, 50, screenHeight-100, color.Gray{Y: 200})
	
	// Draw Stairs chapter
	stairsText := "- Stairs"
	var stairsColor color.Color = color.Gray{Y: 150}
	if g.selectedChapter == ChapterStairs {
		stairsText = "> Stairs"
		stairsColor = color.White
	}
	text.Draw(screen, stairsText, basicfont.Face7x13, 50, screenHeight-80, stairsColor)
	
	// Draw Exit chapter
	exitChapterText := "- Exit"
	var exitChapterColor color.Color = color.Gray{Y: 150}
	if g.selectedChapter == ChapterExit {
		exitChapterText = "> Exit"
		exitChapterColor = color.White
	}
	text.Draw(screen, exitChapterText, basicfont.Face7x13, 50, screenHeight-65, exitChapterColor)
	
	// Draw navigation instructions
	navText := "Use UP/DOWN arrows to select, ENTER/SPACE to activate"
	navBounds := text.BoundString(basicfont.Face7x13, navText)
	navX := (screenWidth - navBounds.Dx()) / 2
	navY := screenHeight - 30
	text.Draw(screen, navText, basicfont.Face7x13, navX, navY, color.Gray{Y: 100})
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

