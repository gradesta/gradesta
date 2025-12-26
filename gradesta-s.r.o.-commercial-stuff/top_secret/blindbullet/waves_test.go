package main

import (
	"math/big"
	"testing"
)

// TestWaveAlignment verifies that wave peaks align with k values from the steps table
// This test is now covered by TestWavePeakAlignment and TestWaveAlignmentAtAllLayers

// TestWavePeakAlignment verifies that wave peaks occur at positions where
// the k value of the next step matches the wave number
func TestWavePeakAlignment(t *testing.T) {
	chapter := NewWavesChapter()
	chapter.globalCoeficient = 3.0
	
	// Generate waves with base multiplier
	waves := chapter.generateWaves(5, 1.0)
	
	// For each wave, check that peaks occur at positions where k matches wave number
	for _, wave := range waves {
		t.Run("Wave_"+wave.Label, func(t *testing.T) {
			// Calculate peak positions for this wave
			// Peaks occur at: center + n*period for integer n
			peakPositions := []int{}
			for n := -5; n <= 5; n++ {
				peakPos := int(float64(wave.Center) + float64(n)*wave.Period)
				peakPositions = append(peakPositions, peakPos)
			}
			
			// For each peak position, verify that the k value of the next step matches the wave number
			for _, peakPos := range peakPositions {
				chapter.currentIndex = big.NewInt(int64(peakPos))
				steps, _, _ := chapter.getCachedStaircase()
				
				if len(steps) > 0 {
					firstStepK := steps[0].K
					expectedK := wave.WaveNum
					if firstStepK != expectedK {
						t.Errorf("Wave %d peak at %d: expected k=%d, got k=%d", 
							wave.WaveNum, peakPos, expectedK, firstStepK)
					}
				} else {
					// If no steps, check the k value directly from the index
					coefBig := big.NewInt(int64(chapter.globalCoeficient))
					yValueBig := new(big.Int).Mul(coefBig, chapter.currentIndex)
					yValueBig.Add(yValueBig, big.NewInt(1))
					k := findLargestPowerOf2Big(yValueBig)
					expectedK := wave.WaveNum
					if k != expectedK {
						t.Errorf("Wave %d peak at %d (no steps): expected k=%d, got k=%d", 
							wave.WaveNum, peakPos, expectedK, k)
					}
				}
			}
		})
	}
}

// TestWavePeakAlignmentWithLayers tests wave alignment at different layers
func TestWavePeakAlignmentWithLayers(t *testing.T) {
	chapter := NewWavesChapter()
	chapter.globalCoeficient = 3.0
	
	// Test base layer
	t.Run("BaseLayer", func(t *testing.T) {
		waves := chapter.generateWaves(5, 1.0)
		verifyWaveAlignment(t, chapter, waves, "base layer")
	})
	
	// Test after up-arrowing through wave 1
	t.Run("Layer1", func(t *testing.T) {
		chapter.upArrowHistory = []WaveHistoryEntry{
			{WaveNum: 1, Center: -1, Period: 4.0},
		}
		periodMultiplier := chapter.calculatePeriodMultiplier()
		waves := chapter.generateWaves(5, periodMultiplier)
		verifyWaveAlignment(t, chapter, waves, "layer 1")
	})
	
	// Test after up-arrowing through wave 1 twice
	t.Run("Layer2", func(t *testing.T) {
		chapter.upArrowHistory = []WaveHistoryEntry{
			{WaveNum: 1, Center: -1, Period: 4.0},
			{WaveNum: 1, Center: -1, Period: 4.0},
		}
		periodMultiplier := chapter.calculatePeriodMultiplier()
		waves := chapter.generateWaves(5, periodMultiplier)
		verifyWaveAlignment(t, chapter, waves, "layer 2")
	})
}

// verifyWaveAlignment checks that wave peaks align with k values
func verifyWaveAlignment(t *testing.T, chapter *WavesChapter, waves []WaveInfo, layerName string) {
	for _, wave := range waves {
		// Calculate a few peak positions
		peakPositions := []int{}
		for n := -3; n <= 3; n++ {
			peakPos := int(float64(wave.Center) + float64(n)*wave.Period)
			peakPositions = append(peakPositions, peakPos)
		}
		
		// Check alignment at each peak
		for _, peakPos := range peakPositions {
			chapter.currentIndex = big.NewInt(int64(peakPos))
			steps, _, _ := chapter.getCachedStaircase()
			
			expectedK := wave.WaveNum
			var actualK int
			
			if len(steps) > 0 {
				actualK = steps[0].K
			} else {
				// If no steps, check the k value directly from the index
				coefBig := big.NewInt(int64(chapter.globalCoeficient))
				yValueBig := new(big.Int).Mul(coefBig, chapter.currentIndex)
				yValueBig.Add(yValueBig, big.NewInt(1))
				actualK = findLargestPowerOf2Big(yValueBig)
			}
			
			if actualK != expectedK {
				t.Errorf("%s - Wave %d peak at %d: expected k=%d, got k=%d", 
					layerName, wave.WaveNum, peakPos, expectedK, actualK)
			}
		}
	}
}

