#!/usr/bin/env python3
"""
Comprehensive test of Tim's Conjecture up to n=100
"""

from TimsConjecture import calculate_N, calculate_D, generate_l_sequences

def comprehensive_test():
    print("Comprehensive test of Tim's Conjecture up to n=100")
    print("="*60)
    
    counterexamples = []
    total_tests = 0
    positive_tests = 0
    
    for n in range(2, 101):
        print(f"\nTesting n={n}")
        
        # For larger n, we need larger l values to get positive D
        # D = 2^(l_0+1) - 3^n, so we need 2^(l_0+1) > 3^n
        # This means l_0+1 > n*log_2(3) ≈ n*1.585
        # So l_0 > n*1.585 - 1
        
        min_l_0 = int(n * 1.585)  # Minimum l_0 to get positive D
        max_l_0 = min_l_0 + 5      # Test a few values above minimum
        
        print(f"  Using l_0 range: {min_l_0} to {max_l_0}")
        
        # Generate sequences with appropriate l_0 values
        sequences_found = 0
        for l_0 in range(min_l_0, max_l_0 + 1):
            # Generate sequences starting with l_0
            for l_1 in range(l_0 + 1, l_0 + 4):  # Keep l values close together
                if n == 2:
                    l_values = [l_0, l_1]
                    sequences_found += 1
                    total_tests += 1
                    
                    N = calculate_N(n, l_values)
                    D = calculate_D(n, l_values[0])
                    
                    if N * D > 0:
                        positive_tests += 1
                        if N % D == 0:
                            counterexamples.append((n, l_values, N, D))
                            print(f"  🚨 COUNTEREXAMPLE: n={n}, l={l_values}, N={N}, D={D}")
                        else:
                            print(f"  ✓ n={n}, l={l_values} → N={N}, D={D} (not divisible)")
                    else:
                        print(f"  ⚠ n={n}, l={l_values} → N={N}, D={D} (N/D not positive)")
                        
                elif n == 3:
                    for l_2 in range(l_1 + 1, l_1 + 3):
                        l_values = [l_0, l_1, l_2]
                        sequences_found += 1
                        total_tests += 1
                        
                        N = calculate_N(n, l_values)
                        D = calculate_D(n, l_values[0])
                        
                        if N * D > 0:
                            positive_tests += 1
                            if N % D == 0:
                                counterexamples.append((n, l_values, N, D))
                                print(f"  🚨 COUNTEREXAMPLE: n={n}, l={l_values}, N={N}, D={D}")
                            else:
                                print(f"  ✓ n={n}, l={l_values} → N={N}, D={D} (not divisible)")
                        else:
                            print(f"  ⚠ n={n}, l={l_values} → N={N}, D={D} (N/D not positive)")
                            
                else:
                    # For n > 3, just test a few representative cases
                    l_values = [l_0 + i for i in range(n)]
                    sequences_found += 1
                    total_tests += 1
                    
                    N = calculate_N(n, l_values)
                    D = calculate_D(n, l_values[0])
                    
                    if N * D > 0:
                        positive_tests += 1
                        if N % D == 0:
                            counterexamples.append((n, l_values, N, D))
                            print(f"  🚨 COUNTEREXAMPLE: n={n}, l={l_values}, N={N}, D={D}")
                        else:
                            print(f"  ✓ n={n}, l={l_values} → N={N}, D={D} (not divisible)")
                    else:
                        print(f"  ⚠ n={n}, l={l_values} → N={N}, D={D} (N/D not positive)")
        
        print(f"  Tested {sequences_found} sequences for n={n}")
        
        # Show progress every 10 n values
        if n % 10 == 0:
            print(f"\n  Progress: n={n}, total_tests={total_tests}, positive_tests={positive_tests}, counterexamples={len(counterexamples)}")
    
    print(f"\n{'='*60}")
    print(f"FINAL SUMMARY")
    print(f"{'='*60}")
    print(f"Total tests: {total_tests}")
    print(f"Tests with positive N/D: {positive_tests}")
    print(f"Counterexamples found: {len(counterexamples)}")
    
    if counterexamples:
        print(f"\nCounterexamples:")
        for n, l_values, N, D in counterexamples:
            print(f"  n={n}, l={l_values} → N={N}, D={D}")
    else:
        print(f"\n✅ Conjecture holds for all tested cases up to n=100!")

if __name__ == "__main__":
    comprehensive_test()
