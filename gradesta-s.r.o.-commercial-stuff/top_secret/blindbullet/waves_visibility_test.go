package main

import (
	"math/big"
	"testing"
)

// TestWaveVisibilityAtSpecificIndices tests that all relevant waves are visible
// at specific indices with specific histories
func TestWaveVisibilityAtSpecificIndices(t *testing.T) {
	testCases := []struct {
		name        string
		index       int64
		history     []WaveHistoryEntry
		expectedWaves []int // Expected wave numbers (k values) that should be visible
		description string
	}{
		{
			name:  "Index1_History2",
			index: 1,
			history: []WaveHistoryEntry{
				{WaveNum: 2, Center: 1, Period: 8.0},
			},
			expectedWaves: []int{1, 2, 3, 4, 5}, // Should see multiple waves like at index -7
			description:   "At index 1 with history [2], should see all relevant waves",
		},
		{
			name:  "IndexMinus7_History2",
			index: -7,
			history: []WaveHistoryEntry{
				{WaveNum: 2, Center: -7, Period: 8.0},
			},
			expectedWaves: []int{1, 2, 3, 4, 5}, // This is the reference case that works
			description:   "At index -7 with history [2], should see all relevant waves (reference)",
		},
		{
			name:  "Index3_History142",
			index: 3,
			history: []WaveHistoryEntry{
				{WaveNum: 1, Center: -1, Period: 4.0},
				{WaveNum: 4, Center: 5, Period: 32.0},
				{WaveNum: 2, Center: 1, Period: 8.0},
			},
			// Waves are only generated for k values that appear as "next k values" at sampled positions
			// With history [1,4,2], historyLen=3, so we look at steps[3].K for the "next k value"
			// The extended steps table provides the k values needed beyond the destination
			// Wave 1 may not appear if it doesn't show up as a "next k value" at any sampled position
			expectedWaves: []int{2, 3, 4, 5, 6, 7, 8}, // Waves that appear as "next k values" at sampled positions
			description:   "At index 3 with history [1,4,2], should see waves for k values that appear as 'next k values'",
		},
	}
	
	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			chapter := NewWavesChapter()
			chapter.globalCoeficient = 3.0
			chapter.currentIndex = big.NewInt(tc.index)
			chapter.upArrowHistory = tc.history
			
			// Generate waves
			periodMultiplier := chapter.calculatePeriodMultiplier()
			waves := chapter.generateWaves(20, periodMultiplier)
			
			// Check that we have waves for all expected k values
			waveMap := make(map[int]bool)
			for _, wave := range waves {
				waveMap[wave.WaveNum] = true
			}
			
			missingWaves := []int{}
			for _, expectedWave := range tc.expectedWaves {
				if !waveMap[expectedWave] {
					missingWaves = append(missingWaves, expectedWave)
				}
			}
			
			if len(missingWaves) > 0 {
				t.Errorf("%s: Missing waves for k values: %v. "+
					"Found waves: %v. "+
					"Expected at least: %v",
					tc.description, missingWaves, getWaveNumbers(waves), tc.expectedWaves)
			}
			
			// Also verify we have a reasonable number of waves
			if len(waves) < 3 {
				t.Errorf("%s: Too few waves generated: %d. Expected at least 3 waves. "+
					"Found waves: %v",
					tc.description, len(waves), getWaveNumbers(waves))
			}
		})
	}
}

// TestWaveVisibilityComparison tests that the same indices with same history
// produce similar wave sets regardless of the specific index value
func TestWaveVisibilityComparison(t *testing.T) {
	chapter1 := NewWavesChapter()
	chapter1.globalCoeficient = 3.0
	chapter1.currentIndex = big.NewInt(1)
	chapter1.upArrowHistory = []WaveHistoryEntry{
		{WaveNum: 2, Center: 1, Period: 8.0},
	}
	
	chapter2 := NewWavesChapter()
	chapter2.globalCoeficient = 3.0
	chapter2.currentIndex = big.NewInt(-7)
	chapter2.upArrowHistory = []WaveHistoryEntry{
		{WaveNum: 2, Center: -7, Period: 8.0},
	}
	
	periodMultiplier1 := chapter1.calculatePeriodMultiplier()
	periodMultiplier2 := chapter2.calculatePeriodMultiplier()
	
	waves1 := chapter1.generateWaves(20, periodMultiplier1)
	waves2 := chapter2.generateWaves(20, periodMultiplier2)
	
	// Get wave numbers for comparison
	waveNums1 := getWaveNumbers(waves1)
	waveNums2 := getWaveNumbers(waves2)
	
	// They should have similar wave sets (at least some overlap)
	// Both should have wave 2 (from history) and several other waves
	hasWave2_1 := false
	hasWave2_2 := false
	for _, w := range waves1 {
		if w.WaveNum == 2 {
			hasWave2_1 = true
			break
		}
	}
	for _, w := range waves2 {
		if w.WaveNum == 2 {
			hasWave2_2 = true
			break
		}
	}
	
	if !hasWave2_1 {
		t.Errorf("Index 1 with history [2]: Missing wave 2. Found waves: %v", waveNums1)
	}
	if !hasWave2_2 {
		t.Errorf("Index -7 with history [2]: Missing wave 2. Found waves: %v", waveNums2)
	}
	
	// Both should have multiple waves
	if len(waves1) < 3 {
		t.Errorf("Index 1 with history [2]: Too few waves: %d. Found: %v", len(waves1), waveNums1)
	}
	if len(waves2) < 3 {
		t.Errorf("Index -7 with history [2]: Too few waves: %d. Found: %v", len(waves2), waveNums2)
	}
	
	// They should have similar number of waves (within reason)
	diff := len(waves1) - len(waves2)
	if diff < 0 {
		diff = -diff
	}
	if diff > 5 {
		t.Errorf("Wave count mismatch: Index 1 has %d waves, Index -7 has %d waves. "+
			"Index 1 waves: %v, Index -7 waves: %v",
			len(waves1), len(waves2), waveNums1, waveNums2)
	}
}

// Helper function to get wave numbers from wave slice
func getWaveNumbers(waves []WaveInfo) []int {
	result := make([]int, len(waves))
	for i, wave := range waves {
		result[i] = wave.WaveNum
	}
	return result
}

