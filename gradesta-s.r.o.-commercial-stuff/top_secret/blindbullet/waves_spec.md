# Wave Generation Specification

## Overview
Waves in the Waves chapter represent k values from the Collatz steps table. Each wave's peaks align with indices where that k value appears as the "next k value" in the steps table. The key insight is that we use the **extended steps table** when we need k values beyond the destination index, ensuring the frequency calculation always uses the same consistent method: multiplying 2^k for all k values in the steps table up to historyLen.

## Wave Peak Alignment

### Rule 1: Wave Peak Position
Each wave peak should align with an index where that wave's k value is the "next k value" for that index.

### Rule 2: "Next k Value" Definition
The "next k value" for an index is determined by:
1. Get the steps table for that index using `CalculateStaircase`
2. If `historyLen >= len(steps)`, extend the steps table using `CalculateStaircaseExtended` with `additionalSteps = historyLen - len(steps) + 10`
3. The "next k value" is at row index = history length in the (possibly extended) steps table
4. If we're still beyond the extended steps (shouldn't happen with proper extension), calculate k directly from the index

**This logic is identical to pressing the up arrow at that index.**

### Rule 3: Wave Generation
For each wave:
- **Wave Number (k)**: The k value that appears as the "next k value" at some index
- **Wave Center**: An index where this k value is the "next k value"
- **Wave Period**: Calculated from frequency (see below)

## Frequency Calculation

### Rule 4: Wave Frequency (The Key Insight)
The frequency for a wave is calculated using **only** the k values from the (extended) steps table:
```
frequency = 2^k_0 * 2^k_1 * 2^k_2 * ... * 2^k_historyLen
```

Where:
- `k_0, k_1, k_2, ..., k_historyLen` are the k values from the steps table rows 0 through historyLen
- The steps table is extended beyond the destination if needed using `CalculateStaircaseExtended`
- **We do NOT include k values from the history directly** - they come from the extended steps table
- **We do NOT include the wave's own k value separately** - it's already in the steps table at row historyLen

**Key Point**: By extending the steps table beyond the destination, we ensure that all k values needed for the frequency calculation are present in the steps table itself. This maintains consistency with the frequency calculation used in other chapters (like Spiral Staircase).

### Rule 5: Wave Period
The period of a wave is:
```
period = 2 * frequency
```

This is because the period is 2x the spacing between peaks. The minimum period is 4.0.

## Extended Steps Table

### Rule 6: When to Use Extended Steps Table
The extended steps table (`CalculateStaircaseExtended`) is used when:
- `historyLen >= len(steps)` (we need k values beyond what the normal staircase provides)
- The extended table continues calculating steps from the destination index
- `additionalSteps = historyLen - len(steps) + 10` provides enough buffer

This ensures that when calculating frequency, we always have enough k values in the steps table to go up to `historyLen`.

## Examples

### Example 1: Base Layer (no history)
- History: [], historyLen = 0
- At index -1: steps table row 0: k = 1
- Next k = 1 (from steps[0].K)
- Wave 1 center: -1
- Wave 1 frequency: 2^1 = 2 (from steps[0].K=1)
- Wave 1 period: 2 * 2 = 4 (minimum period is 4.0, so period = 4.0)

### Example 2: History [2]
- History: [2] (k=2), historyLen = 1
- At index 1 with history [2]:
  - Steps table: row 0: k=1, row 1: k=2
  - Next k = 2 (from steps[1].K)
  - Wave 2 center: 1
  - Wave 2 frequency: 2^1 * 2^2 = 2 * 4 = 8 (from steps[0].K=1 and steps[1].K=2)
  - Wave 2 period: 2 * 8 = 16

- At index -7 with history [2]:
  - Steps table: row 0: k=1, row 1: k=2
  - Next k = 2
  - Wave 2 also peaks at -7 (same k value, different center position)

### Example 3: History [1, 4, 2] at index 3
- History: [1, 4, 2], historyLen = 3
- At index 3:
  - Normal steps table might only have 2 steps (ending at destination)
  - We extend it: `CalculateStaircaseExtended` with `additionalSteps = 3 - 2 + 10 = 11`
  - Extended steps table: row 0: k=1, row 1: k=4, row 2: k=2, row 3: k=7, ...
  - Next k = 7 (from extended steps[3].K)
  - Wave 7 frequency: 2^1 * 2^4 * 2^2 * 2^7 = 2 * 16 * 4 * 128 = 16384
  - Wave 7 period: 2 * 16384 = 32768

## Implementation Notes

1. **Wave Generation Algorithm**:
   - First, generate a wave for the current index's "next k value"
     - Use extended steps table if `historyLen >= len(steps)`
   - Then, iterate through positions on the number line (spaced by layerPeriod)
   - For each position:
     - Get steps table, extend if needed
     - Calculate the "next k value" using Rule 2
     - Skip positions where a wave already peaks (to avoid duplicates)
     - Calculate frequency using only k values from the extended steps table (rows 0 through historyLen)
   - Generate a wave for each unique k value that appears as a "next k value"
   - Only k values that actually appear as "next k values" at sampled positions will have waves generated

2. **Peak Detection**: A wave peaks at index `i` if `(i - wave.Center) % wave.Period == 0`

3. **Consistency**: The logic for determining "next k value" must be identical to the up arrow logic

4. **Frequency Calculation (Critical)**: 
   - **Always use the extended steps table** when `historyLen >= len(steps)`
   - Frequency = product of 2^k for all k values in steps table rows 0 through historyLen
   - Do NOT include k values from history directly
   - Do NOT include the wave's k value separately (it's already in the steps table)
   - This ensures frequency matches the actual Collatz sequence progression

5. **Extended Steps Table**: The `CalculateStaircaseExtended` function continues calculating steps from the destination index, providing the k values needed for frequency calculation beyond the normal staircase endpoint.

