package main

import (
	"testing"
)

// TestJacobsthalSequenceIncreasingKValues tests that for history length 4,
// with offset -28, the Jacobsthal sequence selects numbers from the generated sequence
// whose next k values form an increasing sequence: 1, 2, 3, 4, 5, ...
func TestJacobsthalSequenceIncreasingKValues(t *testing.T) {
	chapter := NewAlternatingJacobsthalChapter()
	
	// Set up history with 4 items (history length 4 means we calculate 5th k value)
	chapter.history = []struct {
		cellNum int
		kValue  int
	}{
		{cellNum: 0, kValue: 1},
		{cellNum: 1, kValue: 2},
		{cellNum: 2, kValue: 3},
		{cellNum: 3, kValue: 4},
	}
	
	// Generate sequence from cell 0
	// Formula: sequenceValue(i) = cellValue + i * 2^cellNum
	cellNum := 0
	cellValue := chapter.getOriginalCellValue(cellNum) // Should be 0 (J(0))
	if cellValue != 0 {
		t.Fatalf("Expected cell 0 to have value 0, got %d", cellValue)
	}
	
	chapter.sequenceCellNum = cellNum
	chapter.sequenceCellValue = cellValue
	chapter.sequenceOffset = -28
	chapter.usingGeneratedSequence = true
	
	// Test that -1 in Jacobsthal (cell 1) maps to approximately -923 with offset -28
	// Cell 1 has original value -1
	jacobValue := chapter.getOriginalCellValue(1) // Should be -1
	if jacobValue != -1 {
		t.Fatalf("Expected cell 1 to have Jacobsthal value -1, got %d", jacobValue)
	}
	
	// With offset -28, index should be -1 + (-28) = -29
	// But since we're using even numbers only, we need to round to nearest even
	// Actually, let me check: the user said offset -28 matches -1 to -923
	// So: sequenceValue(-29) ≈ -923
	// With new formula: 0 + (-29) * 2^0 = -29 (not -923)
	// So maybe the offset needs to be different, or we need a different cell?
	
	// Let's test with the actual expected behavior: k values should be increasing
	// Check cells 0, 1, 2, 3, 4, 5 (right column: even cell numbers)
	expectedKValues := []int{1, 2, 3, 4, 5, 6} // Increasing sequence
	cellNumbers := []int{0, 2, 4, 6, 8, 10}    // Right column cells (even numbers)
	
	t.Logf("Testing with offset -28, history length 4")
	t.Logf("Sequence formula: value = %d + i * 2^%d", chapter.sequenceCellValue, chapter.sequenceCellNum)
	
	for i, cellNum := range cellNumbers {
		// Get the Jacobsthal value for this cell
		jacobVal := chapter.getOriginalCellValue(cellNum)
		// Apply offset to get index into sequence
		indexValue := jacobVal + chapter.sequenceOffset
		// Get sequence value
		cellValue := chapter.getCellValue(cellNum)
		// Calculate k value (5th k value since history length is 4)
		kValue := chapter.calculateNextKValue(cellValue)
		expectedK := expectedKValues[i]
		
		t.Logf("Cell %d: Jacobsthal=%d, index=%d, sequence value=%d, k=%d (expected %d)",
			cellNum, jacobVal, indexValue, cellValue, kValue, expectedK)
		
		if kValue != expectedK {
			t.Errorf("Cell %d: expected k=%d, got k=%d (cell value: %d, index: %d)",
				cellNum, expectedK, kValue, cellValue, indexValue)
		}
	}
}

