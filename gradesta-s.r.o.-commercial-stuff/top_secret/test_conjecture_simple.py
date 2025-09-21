#!/usr/bin/env python3
"""
Simple test of Tim's Conjecture to check the pattern
"""

from TimsConjecture import calculate_N, calculate_D, generate_l_sequences

def quick_test():
    print("Quick test of Tim's Conjecture")
    print("Testing n=2 to n=20 with small l values")
    
    counterexamples = []
    
    for n in range(2, 21):
        print(f"\nTesting n={n}")
        
        # Use small l values for efficiency
        max_l = min(4, n+2)  # Keep l values small
        sequences = generate_l_sequences(n, max_l)
        
        # Test only first few sequences
        test_sequences = sequences[:10] if len(sequences) > 10 else sequences
        
        print(f"  Testing {len(test_sequences)} sequences (max_l={max_l})")
        
        for l_values in test_sequences:
            N = calculate_N(n, l_values)
            D = calculate_D(n, l_values[0])
            
            # Check if N/D is positive
            if N * D > 0:
                # Check divisibility
                if N % D == 0:
                    counterexamples.append((n, l_values, N, D))
                    print(f"  🚨 COUNTEREXAMPLE: n={n}, l={l_values}, N={N}, D={D}")
                else:
                    print(f"  ✓ n={n}, l={l_values} → N={N}, D={D} (not divisible)")
            else:
                print(f"  ⚠ n={n}, l={l_values} → N={N}, D={D} (N/D not positive)")
    
    print(f"\nSUMMARY:")
    print(f"Counterexamples found: {len(counterexamples)}")
    if counterexamples:
        for n, l_values, N, D in counterexamples:
            print(f"  n={n}, l={l_values} → N={N}, D={D}")
    else:
        print("✅ Conjecture holds for all tested cases!")

if __name__ == "__main__":
    quick_test()
