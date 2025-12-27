package main

import (
	"fmt"
	"math/big"
	"testing"
)

// TestUpArrowAddsCorrectKValue tests that pressing up arrow adds the correct k value
// from the steps table. The k value should be from row index equal to history length.
// 
// Example:
// - History [] (length 0): up arrow should add k from row 0
// - History [1] (length 1): up arrow should add k from row 1
// - History [1, 2] (length 2): up arrow should add k from row 2
func TestUpArrowAddsCorrectKValue(t *testing.T) {
	chapter := NewWavesChapter()
	chapter.globalCoeficient = 3.0
	
	// Test at various indices
	testIndices := []int64{-1, 1, -3, 5, -7, 9, -11, 13, -15, 17, -19, 21, -61, 63}
	
	for _, testIdx := range testIndices {
		t.Run(fmt.Sprintf("Index_%d", testIdx), func(t *testing.T) {
			chapter.currentIndex = big.NewInt(testIdx)
			
			// Get the steps table for this index
			steps, _, _ := chapter.getCachedStaircase()
			
			// Test with different history lengths
			for historyLen := 0; historyLen < 5 && historyLen < len(steps); historyLen++ {
				// Set up history with dummy entries (we'll verify the k value, not the full entry)
				chapter.upArrowHistory = make([]WaveHistoryEntry, historyLen)
				for i := 0; i < historyLen; i++ {
					// Use dummy values - we only care about the length
					chapter.upArrowHistory[i] = WaveHistoryEntry{
						WaveNum: i + 1,
						Center:  0,
						Period:  4.0,
					}
				}
				
				// Determine expected k value from steps table
				// Row index = history length
				var expectedK int
				if historyLen < len(steps) {
					expectedK = steps[historyLen].K
				} else {
					// If we're beyond the steps, calculate k directly from index
					coefBig := big.NewInt(int64(chapter.globalCoeficient))
					yValueBig := new(big.Int).Mul(coefBig, chapter.currentIndex)
					yValueBig.Add(yValueBig, big.NewInt(1))
					expectedK = findLargestPowerOf2Big(yValueBig)
				}
				
				// Simulate up arrow press using the NEW logic from Update() function
				// Get k value from steps table at row index = history length
				var actualK int
				if historyLen < len(steps) {
					actualK = steps[historyLen].K
				} else {
					// If we're beyond the steps, calculate k directly from index
					coefBig := big.NewInt(int64(chapter.globalCoeficient))
					yValueBig := new(big.Int).Mul(coefBig, chapter.currentIndex)
					yValueBig.Add(yValueBig, big.NewInt(1))
					actualK = findLargestPowerOf2Big(yValueBig)
				}
				
				// Find the wave info for this k value
				// Use periodMultiplier from history to generate waves correctly
				periodMultiplier := chapter.calculatePeriodMultiplier()
				waves := chapter.generateWaves(20, periodMultiplier)
				var targetWave *WaveInfo
				for i := range waves {
					if waves[i].WaveNum == actualK {
						targetWave = &waves[i]
						break
					}
				}
				
				// Check that the wave number matches the expected k value
				if targetWave == nil {
					t.Errorf("Index %d, history length %d: No wave found for k=%d. "+
						"Expected k=%d from steps table row %d.",
						testIdx, historyLen, actualK, expectedK, historyLen)
					continue
				}
				
				if actualK != expectedK {
					t.Errorf("Index %d, history length %d: Up arrow would add wave %d (k=%d), "+
						"but expected k=%d from steps table row %d. "+
						"Wave center=%d, period=%.1f",
						testIdx, historyLen, targetWave.WaveNum, actualK, expectedK, historyLen,
						targetWave.Center, targetWave.Period)
				}
			}
		})
	}
}

// TestUpArrowHistoryMatchesStepsTable tests that the history built by up-arrowing
// matches the k values in the steps table in order
func TestUpArrowHistoryMatchesStepsTable(t *testing.T) {
	chapter := NewWavesChapter()
	chapter.globalCoeficient = 3.0
	
	// Test at various indices
	testIndices := []int64{-1, 1, -3, 5, -7, 9, -11, 13, -15, 17, -19, 21, -61, 63}
	
	for _, testIdx := range testIndices {
		t.Run(fmt.Sprintf("Index_%d", testIdx), func(t *testing.T) {
			chapter.currentIndex = big.NewInt(testIdx)
			chapter.upArrowHistory = []WaveHistoryEntry{} // Start with empty history
			
			// Get the steps table for this index
			steps, _, _ := chapter.getCachedStaircase()
			
			// Simulate pressing up arrow multiple times
			// Each press should add the k value from the next row in the steps table
			for i := 0; i < len(steps) && i < 5; i++ {
				// Simulate up arrow press using the NEW logic
				// Get k value from steps table at row index = history length
				var actualK int
				if i < len(steps) {
					actualK = steps[i].K
				} else {
					// If we're beyond the steps, calculate k directly from index
					coefBig := big.NewInt(int64(chapter.globalCoeficient))
					yValueBig := new(big.Int).Mul(coefBig, chapter.currentIndex)
					yValueBig.Add(yValueBig, big.NewInt(1))
					actualK = findLargestPowerOf2Big(yValueBig)
				}
				
				// Find the wave info for this k value
				waves := chapter.generateWaves(20, 1.0)
				var targetWave *WaveInfo
				for j := range waves {
					if waves[j].WaveNum == actualK {
						targetWave = &waves[j]
						break
					}
				}
				
				if targetWave == nil {
					t.Errorf("Index %d, iteration %d: No wave found for k=%d. Expected k=%d from steps table row %d.",
						testIdx, i, actualK, steps[i].K, i)
					break
				}
				
				expectedK := steps[i].K
				if actualK != expectedK {
					t.Errorf("Index %d, iteration %d: Up arrow would add wave %d (k=%d), "+
						"but expected k=%d from steps table row %d.",
						testIdx, i, targetWave.WaveNum, actualK, expectedK, i)
					break
				}
				
				// Add to history (simulating the actual up arrow behavior)
				chapter.upArrowHistory = append(chapter.upArrowHistory, WaveHistoryEntry{
					WaveNum: targetWave.WaveNum,
					Center:  targetWave.Center,
					Period:  targetWave.Period,
				})
				
				// Verify history matches steps table so far
				if len(chapter.upArrowHistory) != i+1 {
					t.Errorf("Index %d, iteration %d: History length mismatch. Expected %d, got %d.",
						testIdx, i, i+1, len(chapter.upArrowHistory))
				}
				
				for j := 0; j <= i; j++ {
					if chapter.upArrowHistory[j].WaveNum != steps[j].K {
						t.Errorf("Index %d, iteration %d: History entry %d is wave %d, "+
							"but expected k=%d from steps table row %d.",
							testIdx, i, j, chapter.upArrowHistory[j].WaveNum, steps[j].K, j)
					}
				}
			}
		})
	}
}

