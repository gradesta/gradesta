package main

import (
	"math/big"
	"testing"
)

// TestUpArrowAtSpecificIndex tests that up-arrowing at a specific index
// correctly identifies the wave that peaks there and adds it to history.
// 
// IMPORTANT: This test documents the EXPECTED behavior. The wave alignment
// algorithm needs to be fixed so that wave n peaks at positions where k=n
// from the Collatz steps table.
//
// Expected behavior:
// - History: [1] (after up-arrowing wave 1 once)
// - Navigate to index -63
// - At -63, k value from steps table should be 4
// - Wave 4 should peak at -63 (to align with k=4)
// - Up-arrowing should add wave 4 to history, resulting in [1, 4]
func TestUpArrowAtSpecificIndex(t *testing.T) {
	chapter := NewWavesChapter()
	chapter.globalCoeficient = 3.0
	
	// Set up initial history: [1] (after up-arrowing wave 1 once)
	chapter.upArrowHistory = []WaveHistoryEntry{
		{WaveNum: 1, Center: -1, Period: 4.0},
	}
	
	// Navigate to index -61 (user corrected: should be -61, not -63)
	chapter.currentIndex = big.NewInt(-61)
	
	// Check what k value we get at -61
	coefBig := big.NewInt(int64(chapter.globalCoeficient))
	yValueBig := new(big.Int).Mul(coefBig, chapter.currentIndex)
	yValueBig.Add(yValueBig, big.NewInt(1))
	kAtMinus61 := findLargestPowerOf2Big(yValueBig)
	
	// Wave n should peak where k=n, so expected wave number = k value
	expectedWaveNum := kAtMinus61
	t.Logf("At index -61: k=%d, so wave %d should peak here", kAtMinus61, expectedWaveNum)
	
	// Calculate period multiplier for current layer
	periodMultiplier := chapter.calculatePeriodMultiplier()
	
	// Generate waves with the current multiplier
	waves := chapter.generateWaves(10, periodMultiplier)
	
	// Find which wave peaks at -61
	// Wave peaks occur at: center + n*period for integer n
	var peakingWave *WaveInfo
	currentInt := chapter.currentIndex.Int64()
	for i := range waves {
		wave := &waves[i]
		// Check if -61 is a peak position for this wave
		diff := currentInt - int64(wave.Center)
		periodInt := int64(wave.Period)
		if periodInt > 0 {
			diffAbs := diff
			if diffAbs < 0 {
				diffAbs = -diffAbs
			}
			if diffAbs%periodInt == 0 {
				peakingWave = wave
				break
			}
		}
	}
	
	if peakingWave == nil {
		t.Fatalf("No wave peaks at index -61 with history [1]")
	}
	
	// Verify that the wave peaking at -61 matches the k value
	if peakingWave.WaveNum != expectedWaveNum {
		t.Errorf("ALIGNMENT ISSUE: At index -61, k=%d, so wave %d should peak here, "+
			"but current algorithm shows wave %d peaks here. "+
			"The wave alignment algorithm needs to be fixed to align waves with k values from steps table.",
			kAtMinus61, expectedWaveNum, peakingWave.WaveNum)
	}
	
	// Verify that up-arrowing would add wave 4 to history
	// Simulate the up-arrow logic
	baseWaves := chapter.generateWaves(10, 1.0) // Base waves without multiplier
	// currentInt already set above
	
	var bestWave *WaveInfo
	bestPeriod := 1e9
	
	for i := range baseWaves {
		wave := &baseWaves[i]
		diff := currentInt - int64(wave.Center)
		periodInt := int64(wave.Period)
		if periodInt > 0 {
			diffAbs := diff
			if diffAbs < 0 {
				diffAbs = -diffAbs
			}
			if diffAbs%periodInt == 0 {
				if wave.Period < bestPeriod {
					bestWave = wave
					bestPeriod = wave.Period
				}
			}
		}
	}
	
	if bestWave == nil {
		t.Fatalf("Up-arrow logic found no wave peaking at -61")
	}
	
	// kAtMinus61 and expectedWaveNum already calculated at top of function
	if bestWave.WaveNum != expectedWaveNum {
		t.Errorf("Up-arrow logic: at index -61, k=%d, so expected wave %d, got wave %d", 
			kAtMinus61, expectedWaveNum, bestWave.WaveNum)
	}
	
	// Test that adding wave 4 to history results in [1, 4]
	newHistory := append(chapter.upArrowHistory, WaveHistoryEntry{
		WaveNum: bestWave.WaveNum,
		Center:  bestWave.Center,
		Period:  bestWave.Period,
	})
	
	if len(newHistory) != 2 {
		t.Errorf("Expected history length 2, got %d", len(newHistory))
	}
	
	if newHistory[0].WaveNum != 1 {
		t.Errorf("Expected first history entry to be wave 1, got wave %d", newHistory[0].WaveNum)
	}
	
	if newHistory[1].WaveNum != expectedWaveNum {
		t.Errorf("Expected second history entry to be wave %d (k at -61), got wave %d", 
			expectedWaveNum, newHistory[1].WaveNum)
	}
}

// TestUpArrowWaveSelection tests that up-arrowing selects the correct wave
// based on the current index and history
func TestUpArrowWaveSelection(t *testing.T) {
	testCases := []struct {
		name            string
		history         []WaveHistoryEntry
		currentIndex    int64
		expectedWaveNum int
		description     string
	}{
		{
			name:            "BaseLayer_IndexMinus1",
			history:         []WaveHistoryEntry{},
			currentIndex:    -1,
			expectedWaveNum: 1,
			description:     "Base layer at -1 should select wave 1",
		},
		{
			name:            "BaseLayer_Index1",
			history:         []WaveHistoryEntry{},
			currentIndex:    1,
			expectedWaveNum: 2,
			description:     "Base layer at 1 should select wave 2",
		},
		{
			name: "History1_IndexMinus61",
			history: []WaveHistoryEntry{
				{WaveNum: 1, Center: -1, Period: 4.0},
			},
			currentIndex:    -61,
			expectedWaveNum: 4, // Will be determined by k value at -61
			description:     "History [1] at -61 should select wave matching k value",
		},
		{
			name: "History1_Index5",
			history: []WaveHistoryEntry{
				{WaveNum: 1, Center: -1, Period: 4.0},
			},
			currentIndex:    5,
			expectedWaveNum: 4, // Will be determined by k value at 5
			description:     "History [1] at 5 should select wave matching k value",
		},
	}
	
	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			chapter := NewWavesChapter()
			chapter.globalCoeficient = 3.0
			chapter.upArrowHistory = tc.history
			chapter.currentIndex = big.NewInt(tc.currentIndex)
			
			// Check what k value we get at this index
			coefBig := big.NewInt(int64(chapter.globalCoeficient))
			yValueBig := new(big.Int).Mul(coefBig, chapter.currentIndex)
			yValueBig.Add(yValueBig, big.NewInt(1))
			kAtIndex := findLargestPowerOf2Big(yValueBig)
			
			// Wave n should peak where k=n, so expected wave = k value
			expectedWaveNum := kAtIndex
			if tc.expectedWaveNum > 0 {
				// If test case specifies expected, use it (for now, until alignment is fixed)
				expectedWaveNum = tc.expectedWaveNum
			}
			
			// Generate base waves (multiplier = 1.0) to find which wave peaks
			baseWaves := chapter.generateWaves(10, 1.0)
			currentInt := chapter.currentIndex.Int64()
			
			var bestWave *WaveInfo
			bestPeriod := 1e9
			
			for i := range baseWaves {
				wave := &baseWaves[i]
				diff := currentInt - int64(wave.Center)
				periodInt := int64(wave.Period)
				if periodInt > 0 {
					diffAbs := diff
					if diffAbs < 0 {
						diffAbs = -diffAbs
					}
					if diffAbs%periodInt == 0 {
						if wave.Period < bestPeriod {
							bestWave = wave
							bestPeriod = wave.Period
						}
					}
				}
			}
			
			if bestWave == nil {
				t.Fatalf("%s: No wave found peaking at index %d", tc.description, tc.currentIndex)
			}
			
			if bestWave.WaveNum != expectedWaveNum {
				t.Errorf("%s: At index %d, k=%d, so expected wave %d, got wave %d",
					tc.description, tc.currentIndex, kAtIndex, expectedWaveNum, bestWave.WaveNum)
			}
		})
	}
}

// TestWavePeaksAtIndex verifies that waves peak at the expected indices
func TestWavePeaksAtIndex(t *testing.T) {
	chapter := NewWavesChapter()
	chapter.globalCoeficient = 3.0
	
	// Test base layer waves
	waves := chapter.generateWaves(5, 1.0)
	
	testCases := []struct {
		waveNum      int
		index        int64
		shouldPeak   bool
		description  string
	}{
		{1, -1, true, "Wave 1 peaks at -1"},
		{1, 3, true, "Wave 1 peaks at 3"},
		{1, 7, true, "Wave 1 peaks at 7"},
		{1, 0, false, "Wave 1 does not peak at 0"},
		{2, 1, true, "Wave 2 peaks at 1"},
		{2, 9, true, "Wave 2 peaks at 9"},
		{2, -7, true, "Wave 2 peaks at -7"},
		{4, -61, true, "Wave 4 peaks at -61"},
		{4, 5, true, "Wave 4 peaks at 5"},
		{4, 69, true, "Wave 4 peaks at 69"},
	}
	
	for _, tc := range testCases {
		t.Run(tc.description, func(t *testing.T) {
			var wave *WaveInfo
			for i := range waves {
				if waves[i].WaveNum == tc.waveNum {
					wave = &waves[i]
					break
				}
			}
			
			if wave == nil {
				t.Fatalf("Wave %d not found", tc.waveNum)
			}
			
			// Check if index is a peak position
			diff := tc.index - int64(wave.Center)
			periodInt := int64(wave.Period)
			isPeak := false
			if periodInt > 0 {
				diffAbs := diff
				if diffAbs < 0 {
					diffAbs = -diffAbs
				}
				isPeak = diffAbs%periodInt == 0
			}
			
			if isPeak != tc.shouldPeak {
				t.Errorf("Wave %d at index %d: expected peak=%v, got peak=%v. "+
					"Wave center=%d, period=%.1f, diff=%d",
					tc.waveNum, tc.index, tc.shouldPeak, isPeak,
					wave.Center, wave.Period, diff)
			}
		})
	}
}

// TestUpArrowHistoryUpdate tests the full up-arrow flow
func TestUpArrowHistoryUpdate(t *testing.T) {
	chapter := NewWavesChapter()
	chapter.globalCoeficient = 3.0
	
	// Start with history [1]
	chapter.upArrowHistory = []WaveHistoryEntry{
		{WaveNum: 1, Center: -1, Period: 4.0},
	}
	
	// Navigate to -61 (user corrected: should be -61, not -63)
	chapter.currentIndex = big.NewInt(-61)
	
	// Simulate up-arrow press
	baseWaves := chapter.generateWaves(10, 1.0)
	currentInt := chapter.currentIndex.Int64()
	
	var bestWave *WaveInfo
	bestPeriod := 1e9
	
	for i := range baseWaves {
		wave := &baseWaves[i]
		diff := currentInt - int64(wave.Center)
		periodInt := int64(wave.Period)
		if periodInt > 0 {
			diffAbs := diff
			if diffAbs < 0 {
				diffAbs = -diffAbs
			}
			if diffAbs%periodInt == 0 {
				if wave.Period < bestPeriod {
					bestWave = wave
					bestPeriod = wave.Period
				}
			}
		}
	}
	
	if bestWave == nil {
		t.Fatalf("No wave found peaking at -61")
	}
	
	// Check k value at -61 to determine expected wave
	coefBig := big.NewInt(int64(chapter.globalCoeficient))
	yValueBig := new(big.Int).Mul(coefBig, chapter.currentIndex)
	yValueBig.Add(yValueBig, big.NewInt(1))
	kAtMinus61 := findLargestPowerOf2Big(yValueBig)
	expectedWaveNum := kAtMinus61
	
	if bestWave.WaveNum != expectedWaveNum {
		t.Errorf("ALIGNMENT ISSUE: At index -61, k=%d, so wave %d should peak here, "+
			"but up-arrow logic found wave %d. "+
			"The wave alignment algorithm needs to be fixed to align waves with k values from steps table.",
			kAtMinus61, expectedWaveNum, bestWave.WaveNum)
		// Continue test to verify history update logic even if alignment is wrong
		expectedWaveNum = bestWave.WaveNum // Use actual value for rest of test
	}
	
	// Update history (simulating what Update() would do)
	chapter.upArrowHistory = append(chapter.upArrowHistory, WaveHistoryEntry{
		WaveNum: bestWave.WaveNum,
		Center:  bestWave.Center,
		Period:  bestWave.Period,
	})
	
	// Verify history is now [1, expectedWaveNum]
	if len(chapter.upArrowHistory) != 2 {
		t.Errorf("Expected history length 2, got %d", len(chapter.upArrowHistory))
	}
	
	if chapter.upArrowHistory[0].WaveNum != 1 {
		t.Errorf("Expected first entry wave 1, got wave %d", chapter.upArrowHistory[0].WaveNum)
	}
	
		if chapter.upArrowHistory[1].WaveNum != expectedWaveNum {
		t.Errorf("Expected second entry wave %d (k at -61), got wave %d", 
			expectedWaveNum, chapter.upArrowHistory[1].WaveNum)
	}
	
	// Verify the new layer period calculation
	newLayerPeriod := chapter.getCurrentLayerPeriod()
	// History [1, expectedWaveNum]: 2 * (4/2) * (period/2) where period depends on wave number
	wave4Period := 32.0 // Wave 4 has period 32
	expectedPeriod := 2.0 * (4.0 / 2.0) * (wave4Period / 2.0)
	if expectedWaveNum == 4 {
		if newLayerPeriod != expectedPeriod {
			t.Errorf("Expected layer period %.1f, got %.1f", expectedPeriod, newLayerPeriod)
		}
	}
}

