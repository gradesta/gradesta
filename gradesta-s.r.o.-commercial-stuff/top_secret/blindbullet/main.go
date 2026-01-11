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
	StateMaxwellsDaemon
	StateCrystals
	StateStairs
	StateSpiralStairs
	StateThreeRowBootlace
	StateZigZag
	StateGrammarForFiniteSentences
	StateWaves
	StateTable
	StateAlternatingJacobsthal
	StateSchwingerLimit
	StateEssenceOfSelf
)

// Chapter represents a selectable chapter
type Chapter int

const (
	ChapterMaxwellsDaemon Chapter = iota
	ChapterCrystals
	ChapterStairs
	ChapterSpiralStairs
	ChapterThreeRowBootlace
	ChapterZigZag
	ChapterGrammarForFiniteSentences
	ChapterWaves
	ChapterTable
	ChapterAlternatingJacobsthal
	ChapterSchwingerLimit
	ChapterEssenceOfSelf
	ChapterExit
	ChapterCount // Total number of chapters
)

// Game is the main game struct
type Game struct {
	state                        GameState
	maxwellsDaemon               *MaxwellsDaemonChapter
	crystals                     *CrystalsChapter
	stairs                       *StairsChapter
	spiralStairs                 *SpiralStairsChapter
	threeRowBootlace             *ThreeRowBootlaceChapter
	waves                        *WavesChapter
	table                        *TableChapter
	alternatingJacobsthal        *AlternatingJacobsthalChapter
	zigZag                       *ZigZagChapter
	grammarForFiniteSentences    *GrammarForFiniteSentencesChapter
	schwingerLimit               *SchwingerLimitChapter
	essenceOfSelf                *EssenceOfSelfChapter
	selectedChapter              Chapter
}

// NewGame creates a new game instance
func NewGame() *Game {
	return &Game{
		state:                        StateLaunchScreen,
		maxwellsDaemon:               NewMaxwellsDaemonChapter(),
		crystals:                     NewCrystalsChapter(),
		stairs:                       NewStairsChapter(),
		spiralStairs:                 NewSpiralStairsChapter(),
		threeRowBootlace:             NewThreeRowBootlaceChapter(),
		waves:                        NewWavesChapter(),
		table:                        NewTableChapter(),
		alternatingJacobsthal:        NewAlternatingJacobsthalChapter(),
		zigZag:                       NewZigZagChapter(),
		grammarForFiniteSentences:    NewGrammarForFiniteSentencesChapter(),
		schwingerLimit:               NewSchwingerLimitChapter(),
		essenceOfSelf:                NewEssenceOfSelfChapter(),
		selectedChapter:              ChapterMaxwellsDaemon,
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
			case ChapterMaxwellsDaemon:
				g.state = StateMaxwellsDaemon
			case ChapterCrystals:
				g.state = StateCrystals
			case ChapterStairs:
				g.state = StateStairs
			case ChapterSpiralStairs:
				g.state = StateSpiralStairs
			case ChapterThreeRowBootlace:
				g.state = StateThreeRowBootlace
		case ChapterWaves:
			g.state = StateWaves
		case ChapterTable:
			g.state = StateTable
		case ChapterAlternatingJacobsthal:
				g.state = StateAlternatingJacobsthal
			case ChapterZigZag:
				g.state = StateZigZag
			case ChapterGrammarForFiniteSentences:
				g.state = StateGrammarForFiniteSentences
			case ChapterSchwingerLimit:
				g.state = StateSchwingerLimit
			case ChapterEssenceOfSelf:
				g.state = StateEssenceOfSelf
			case ChapterExit:
				return errors.New("user requested exit")
			}
		}
		
		// Allow Escape or Q to exit the application
		if inpututil.IsKeyJustPressed(ebiten.KeyEscape) || inpututil.IsKeyJustPressed(ebiten.KeyQ) {
			return errors.New("user requested exit")
		}
	case StateMaxwellsDaemon:
		if err := g.maxwellsDaemon.Update(); err != nil {
			return err
		}
		// Check if player chose to go to next chapter
		if g.maxwellsDaemon.ShouldGoToNextChapter() {
			g.state = StateCrystals
			// Reset Maxwell's Daemon chapter for next time
			g.maxwellsDaemon = NewMaxwellsDaemonChapter()
		}
		// Check if chapter is complete (player chose to return to menu after heat death)
		if g.maxwellsDaemon.IsChapterComplete() {
			g.state = StateLaunchScreen
			// Reset the chapter for next time
			g.maxwellsDaemon = NewMaxwellsDaemonChapter()
		}
		if inpututil.IsKeyJustPressed(ebiten.KeyEscape) && !g.maxwellsDaemon.WasEscConsumed() {
			g.state = StateLaunchScreen
		}
	case StateCrystals:
		if err := g.crystals.Update(); err != nil {
			return err
		}
		if inpututil.IsKeyJustPressed(ebiten.KeyEscape) && !g.crystals.WasEscConsumed() {
			g.state = StateLaunchScreen
		}
	case StateThreeRowBootlace:
		if err := g.threeRowBootlace.Update(); err != nil {
			return err
		}
		if inpututil.IsKeyJustPressed(ebiten.KeyEscape) && !g.threeRowBootlace.WasEscConsumed() {
			g.state = StateLaunchScreen
		}
	case StateWaves:
		if err := g.waves.Update(); err != nil {
			return err
		}
		if inpututil.IsKeyJustPressed(ebiten.KeyEscape) && !g.waves.WasEscConsumed() {
			g.state = StateLaunchScreen
		}
	case StateTable:
		if err := g.table.Update(); err != nil {
			return err
		}
		if inpututil.IsKeyJustPressed(ebiten.KeyEscape) && !g.table.WasEscConsumed() {
			g.state = StateLaunchScreen
		}
	case StateAlternatingJacobsthal:
		if err := g.alternatingJacobsthal.Update(); err != nil {
			return err
		}
		if inpututil.IsKeyJustPressed(ebiten.KeyEscape) && !g.alternatingJacobsthal.WasEscConsumed() {
			g.state = StateLaunchScreen
		}
	case StateGrammarForFiniteSentences:
		if err := g.grammarForFiniteSentences.Update(); err != nil {
			return err
		}
		if inpututil.IsKeyJustPressed(ebiten.KeyEscape) && !g.grammarForFiniteSentences.WasEscConsumed() {
			g.state = StateLaunchScreen
		}
	case StateSchwingerLimit:
		if err := g.schwingerLimit.Update(); err != nil {
			return err
		}
		if inpututil.IsKeyJustPressed(ebiten.KeyEscape) && !g.schwingerLimit.WasEscConsumed() {
			g.state = StateLaunchScreen
		}
	case StateEssenceOfSelf:
		if err := g.essenceOfSelf.Update(); err != nil {
			return err
		}
		if inpututil.IsKeyJustPressed(ebiten.KeyEscape) && !g.essenceOfSelf.WasEscConsumed() {
			g.state = StateLaunchScreen
		}
	case StateStairs:
		if err := g.stairs.Update(); err != nil {
			return err
		}
		// Only return to home if ESC was pressed and not consumed by a dialog
		if inpututil.IsKeyJustPressed(ebiten.KeyEscape) && !g.stairs.WasEscConsumed() {
			g.state = StateLaunchScreen
		}
	case StateSpiralStairs:
		if err := g.spiralStairs.Update(); err != nil {
			return err
		}
		// Only return to home if ESC was pressed and not consumed by a dialog
		if inpututil.IsKeyJustPressed(ebiten.KeyEscape) && !g.spiralStairs.WasEscConsumed() {
			g.state = StateLaunchScreen
		}
	case StateZigZag:
		if err := g.zigZag.Update(); err != nil {
			return err
		}
		// Only return to home if ESC was pressed and not consumed by a dialog
		if inpututil.IsKeyJustPressed(ebiten.KeyEscape) && !g.zigZag.WasEscConsumed() {
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
	case StateMaxwellsDaemon:
		g.maxwellsDaemon.Draw(screen)
	case StateCrystals:
		g.crystals.Draw(screen)
	case StateStairs:
		g.stairs.Draw(screen)
	case StateSpiralStairs:
		g.spiralStairs.Draw(screen)
	case StateThreeRowBootlace:
		g.threeRowBootlace.Draw(screen)
	case StateWaves:
		g.waves.Draw(screen)
	case StateTable:
		g.table.Draw(screen)
	case StateAlternatingJacobsthal:
		g.alternatingJacobsthal.Draw(screen)
	case StateZigZag:
		g.zigZag.Draw(screen)
	case StateGrammarForFiniteSentences:
		g.grammarForFiniteSentences.Draw(screen)
	case StateSchwingerLimit:
		g.schwingerLimit.Draw(screen)
	case StateEssenceOfSelf:
		g.essenceOfSelf.Draw(screen)
	}
}

func (g *Game) drawLaunchScreen(screen *ebiten.Image) {
	// Draw title (moved up)
	titleText := "BLIND BULLET"
	titleBounds := text.BoundString(basicfont.Face7x13, titleText)
	titleX := (screenWidth - titleBounds.Dx()) / 2
	titleY := 80
	text.Draw(screen, titleText, basicfont.Face7x13, titleX, titleY, color.White)

	// Draw chapter list (left-aligned, moved down)
	chapterText := "Chapters:"
	chapterTextX := 100
	chapterTextY := 150
	text.Draw(screen, chapterText, basicfont.Face7x13, chapterTextX, chapterTextY, color.Gray{Y: 200})
	
	// Draw chapters in order (left-aligned, starting below the "Chapters:" label)
	chapterY := chapterTextY + 20
	chapters := []struct {
		name    string
		chapter Chapter
	}{
		{"Maxwell's Daemon", ChapterMaxwellsDaemon},
		{"Crystals", ChapterCrystals},
		{"Stairs", ChapterStairs},
		{"The Spiral Staircase", ChapterSpiralStairs},
		{"Three Row Bootlace", ChapterThreeRowBootlace},
		{"Zig Zag", ChapterZigZag},
		{"A Grammar for Finite Sentences", ChapterGrammarForFiniteSentences},
		{"Waves", ChapterWaves},
		{"Table", ChapterTable},
		{"The Alternating Jacobsthal sequence", ChapterAlternatingJacobsthal},
		{"The Schwinger Limit", ChapterSchwingerLimit},
		{"The Essence of the Self", ChapterEssenceOfSelf},
		{"Exit", ChapterExit},
	}
	
	for _, ch := range chapters {
		// Left-align chapters
		drawChapter(screen, ch.name, ch.chapter, g.selectedChapter, chapterTextX, chapterY)
		chapterY += 15
	}
	
	// Draw instructions (moved down)
	instructions := "Press ENTER or SPACE to start"
	instBounds := text.BoundString(basicfont.Face7x13, instructions)
	instX := (screenWidth - instBounds.Dx()) / 2
	instY := screenHeight - 60
	text.Draw(screen, instructions, basicfont.Face7x13, instX, instY, color.Gray{Y: 150})
	
	// Draw exit instruction (moved down)
	exitText := "Press ESC or Q to exit"
	exitBounds := text.BoundString(basicfont.Face7x13, exitText)
	exitX := (screenWidth - exitBounds.Dx()) / 2
	exitY := screenHeight - 45
	text.Draw(screen, exitText, basicfont.Face7x13, exitX, exitY, color.Gray{Y: 100})
	
	// Draw navigation instructions (moved down)
	navText := "Use UP/DOWN arrows to select, ENTER/SPACE to activate"
	navBounds := text.BoundString(basicfont.Face7x13, navText)
	navX := (screenWidth - navBounds.Dx()) / 2
	navY := screenHeight - 30
	text.Draw(screen, navText, basicfont.Face7x13, navX, navY, color.Gray{Y: 100})
}

// drawChapter draws a chapter name with selection highlighting (left-aligned)
func drawChapter(screen *ebiten.Image, name string, chapter Chapter, selectedChapter Chapter, x, y int) {
	textStr := "- " + name
	var textColor color.Color = color.Gray{Y: 150}
	if selectedChapter == chapter {
		textStr = "> " + name
		textColor = color.White
	}
	// x is the left-aligned position
	text.Draw(screen, textStr, basicfont.Face7x13, x, y, textColor)
}

// drawTODO draws a TODO placeholder screen
func drawTODO(screen *ebiten.Image, chapterName string) {
	// Draw chapter name
	titleText := chapterName
	titleBounds := text.BoundString(basicfont.Face7x13, titleText)
	titleX := (screenWidth - titleBounds.Dx()) / 2
	titleY := screenHeight/2 - 20
	text.Draw(screen, titleText, basicfont.Face7x13, titleX, titleY, color.White)
	
	// Draw TODO text
	todoText := "TODO"
	todoBounds := text.BoundString(basicfont.Face7x13, todoText)
	todoX := (screenWidth - todoBounds.Dx()) / 2
	todoY := screenHeight/2 + 10
	text.Draw(screen, todoText, basicfont.Face7x13, todoX, todoY, color.Gray{Y: 150})
	
	// Draw instructions
	instructions := "Press ESC to return"
	instBounds := text.BoundString(basicfont.Face7x13, instructions)
	instX := (screenWidth - instBounds.Dx()) / 2
	instY := screenHeight/2 + 30
	text.Draw(screen, instructions, basicfont.Face7x13, instX, instY, color.Gray{Y: 100})
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

