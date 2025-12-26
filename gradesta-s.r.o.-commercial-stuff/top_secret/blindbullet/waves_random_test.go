package main

import (
	"fmt"
	"math/big"
	"math/rand"
	"testing"
	"time"
)

// TestWavePeaksMatchKValuesRandom tests that wave peaks align with k values
// by checking hundreds of random indices
func TestWavePeaksMatchKValuesRandom(t *testing.T) {
	chapter := NewWavesChapter()
	chapter.globalCoeficient = 3.0
	
	// Test different layer configurations
	layerConfigs := []struct {
		name    string
		history []WaveHistoryEntry
	}{
		{"BaseLayer", []WaveHistoryEntry{}},
		{"Layer1_Wave1", []WaveHistoryEntry{{WaveNum: 1, Center: -1, Period: 4.0}}},
		{"Layer2_Wave1x2", []WaveHistoryEntry{
			{WaveNum: 1, Center: -1, Period: 4.0},
			{WaveNum: 1, Center: -1, Period: 4.0},
		}},
	}
	
	rand.Seed(time.Now().UnixNano())
	numTests := 300
	
	for _, config := range layerConfigs {
		t.Run(config.name, func(t *testing.T) {
			chapter.upArrowHistory = config.history
			periodMultiplier := chapter.calculatePeriodMultiplier()
			waves := chapter.generateWaves(10, periodMultiplier)
			
			failures := 0
			maxFailures := 10 // Only report first 10 failures per layer
			
			for i := 0; i < numTests; i++ {
				// Generate random index in a reasonable range
				// Use range that covers both positive and negative values
				index := rand.Int63n(2000) - 1000 // Range: -1000 to 999
				
				chapter.currentIndex = big.NewInt(index)
				
				// Calculate k value at this index
				coefBig := big.NewInt(int64(chapter.globalCoeficient))
				yValueBig := new(big.Int).Mul(coefBig, chapter.currentIndex)
				yValueBig.Add(yValueBig, big.NewInt(1))
				kAtIndex := findLargestPowerOf2Big(yValueBig)
				
				// Skip if k is 0 (no wave for k=0)
				if kAtIndex == 0 {
					continue
				}
				
				// Find which wave peaks at this index
				var peakingWave *WaveInfo
				currentInt := chapter.currentIndex.Int64()
				
				for waveIdx := range waves {
					wave := &waves[waveIdx]
					diff := currentInt - int64(wave.Center)
					periodInt := int64(wave.Period)
					if periodInt > 0 {
						diffAbs := diff
						if diffAbs < 0 {
							diffAbs = -diffAbs
						}
						if diffAbs%periodInt == 0 {
							// This wave peaks at this index
							if peakingWave == nil || wave.Period < peakingWave.Period {
								peakingWave = wave
							}
						}
					}
				}
				
				// Check alignment: wave n should peak where k=n
				if peakingWave != nil {
					if peakingWave.WaveNum != kAtIndex {
						if failures < maxFailures {
							t.Errorf("Index %d: k=%d, but wave %d peaks here (center=%d, period=%.1f). "+
								"Expected wave %d to peak here.",
								index, kAtIndex, peakingWave.WaveNum, peakingWave.Center, peakingWave.Period, kAtIndex)
						}
						failures++
					}
				} else {
					// No wave peaks here, but we have k > 0, so there should be a wave
					if failures < maxFailures {
						t.Errorf("Index %d: k=%d, but no wave peaks here. "+
							"Expected wave %d to peak here.",
							index, kAtIndex, kAtIndex)
					}
					failures++
				}
			}
			
			if failures > 0 {
				t.Errorf("Failed %d out of %d random tests in %s", failures, numTests, config.name)
			} else {
				t.Logf("Passed all %d random tests in %s", numTests, config.name)
			}
		})
	}
}

// TestWavePeaksMatchKValuesSystematic tests wave alignment systematically
// by checking indices where we know k values should be specific values
func TestWavePeaksMatchKValuesSystematic(t *testing.T) {
	chapter := NewWavesChapter()
	chapter.globalCoeficient = 3.0
	
	// Test cases: just indices, we'll calculate expected k from actual values
	testIndices := []int64{
		-1, 1, 3, 5, 7, 9,
		-3, -5, -7, -9, -11, -13, -15, -19, -27, -35, -43, -51, -59, -61, -67, -75, -83, -91, -99,
	}
	
	for _, index := range testIndices {
		t.Run(fmt.Sprintf("Index_%d", index), func(t *testing.T) {
			chapter.currentIndex = big.NewInt(index)
			
			// Calculate actual k value
			coefBig := big.NewInt(int64(chapter.globalCoeficient))
			yValueBig := new(big.Int).Mul(coefBig, chapter.currentIndex)
			yValueBig.Add(yValueBig, big.NewInt(1))
			actualK := findLargestPowerOf2Big(yValueBig)
			
			// Skip if k is 0
			if actualK == 0 {
				return
			}
			
			// Generate waves and find which one peaks here
			waves := chapter.generateWaves(10, 1.0)
			currentInt := chapter.currentIndex.Int64()
			
			var peakingWave *WaveInfo
			for waveIdx := range waves {
				wave := &waves[waveIdx]
				diff := currentInt - int64(wave.Center)
				periodInt := int64(wave.Period)
				if periodInt > 0 {
					diffAbs := diff
					if diffAbs < 0 {
						diffAbs = -diffAbs
					}
					if diffAbs%periodInt == 0 {
						if peakingWave == nil || wave.Period < peakingWave.Period {
							peakingWave = wave
						}
					}
				}
			}
			
			// Verify alignment: wave n should peak where k=n
			if peakingWave == nil {
				t.Errorf("Index %d: k=%d, but no wave peaks here. Expected wave %d to peak here.",
					index, actualK, actualK)
			} else if peakingWave.WaveNum != actualK {
				t.Errorf("Index %d: k=%d, but wave %d peaks here (center=%d, period=%.1f). "+
					"Expected wave %d to peak here.",
					index, actualK, peakingWave.WaveNum, peakingWave.Center, peakingWave.Period, actualK)
			}
		})
	}
}

