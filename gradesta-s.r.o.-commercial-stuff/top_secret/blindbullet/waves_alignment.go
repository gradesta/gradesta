package main

import (
	"math/big"
)

// findWaveCenterForK finds a center position for wave n (where k=n)
// by finding an index where k equals the wave number
// This ensures wave n peaks at positions where k=n
func findWaveCenterForK(waveNum int, coefficient float64) int {
	// Try a range of indices to find one where k = waveNum
	// Start from a reasonable range based on the wave number
	// For higher wave numbers, we might need a larger range
	startIdx := -2000
	endIdx := 2000
	
	coefBig := big.NewInt(int64(coefficient))
	one := big.NewInt(1)
	
	// Try to find the first (most negative) index where k = waveNum
	// This gives us a consistent center for each wave
	for idx := startIdx; idx <= endIdx; idx++ {
		indexBig := big.NewInt(int64(idx))
		// Calculate y = coefficient * index + 1
		yValueBig := new(big.Int).Mul(coefBig, indexBig)
		yValueBig.Add(yValueBig, one)
		
		// Find k
		k := findLargestPowerOf2Big(yValueBig)
		
		if k == waveNum {
			return idx
		}
	}
	
	// Fallback: use the original pattern if we can't find a match
	// This shouldn't happen for reasonable wave numbers
	return calculateFallbackCenter(waveNum)
}

// calculateFallbackCenter calculates center using the original pattern
// as a fallback if we can't find a k match
func calculateFallbackCenter(waveNum int) int {
	if waveNum == 1 {
		return -1
	}
	
	center := -1
	period := 4.0
	
	for i := 1; i < waveNum; i++ {
		halfPeriod := int(period / 2)
		if i%2 == 1 {
			center = center + halfPeriod
		} else {
			center = center - halfPeriod
		}
		period *= 2.0
	}
	
	return center
}

