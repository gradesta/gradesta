#!/usr/bin/env python3
"""
Tim's Conjecture Tester

Tests the conjecture that:
N = 2*3^n + sum(i=1 to n) 3^(n-i) * 2^(l_i) - 2^(l_0)
is NOT divisible by D = 2^(l_0+1) - 3^n

where:
- n is a positive integer
- l_0, l_1, ..., l_n are non-negative integers
- l_i > l_(i+1) for all i (strictly decreasing sequence)
"""

import math
import random
from itertools import combinations_with_replacement, permutations
from collections import defaultdict


def prime_factors(n):
    """Return prime factorization of n as a dictionary {prime: exponent}"""
    if n == 0:
        return {0: 1}
    if n == 1:
        return {}
    
    factors = {}
    # Handle negative numbers
    if n < 0:
        factors[-1] = 1
        n = -n
    
    # Check for 2
    while n % 2 == 0:
        factors[2] = factors.get(2, 0) + 1
        n //= 2
    
    # Check odd numbers up to sqrt(n)
    i = 3
    while i * i <= n:
        while n % i == 0:
            factors[i] = factors.get(i, 0) + 1
            n //= i
        i += 2
    
    # If n is still > 1, it's a prime
    if n > 1:
        factors[n] = factors.get(n, 0) + 1
    
    return factors


def format_prime_factors(factors):
    """Format prime factorization as a readable string"""
    if not factors:
        return "1"
    
    terms = []
    for prime, exp in sorted(factors.items()):
        if exp == 1:
            terms.append(str(prime))
        else:
            terms.append(f"{prime}^{exp}")
    
    return " × ".join(terms)


def calculate_N(n, l_values):
    """
    Calculate N = 2*3^n + sum(i=1 to n) 3^(n-i) * 2^(l_i) - 2^(l_0)
    
    Args:
        n: positive integer
        l_values: list of n non-negative integers [l_0, l_1, ..., l_{n-1}]
    """
    # First term: 2 * 3^n
    N = 2 * (3 ** n)

    # Sum term: sum(i=1 to n) 3^(n-i) * 2^(l_i)
    for i in range(1, n):
        t = (3 ** (n - i)) * (2 ** l_values[i])
        N += t
    
    # Subtract: 2^(l_0)
    t = 2 ** l_values[0]
    N -= t
    return N


def calculate_D(n, l_0):
    """
    Calculate D = 2^(l_0+1) - 3^n
    """
    return (2 ** (l_0 + 1)) - (3 ** n)


def is_valid_l_sequence(l_values):
    """Check if l_values form a strictly increasing sequence"""
    for i in range(len(l_values) - 1):
        if l_values[i] >= l_values[i + 1]:
            return False
    return True


def generate_l_sequences(n, max_l_value=10):
    """
    Generate valid l sequences for given n.
    Returns list of tuples (l_0, l_1, ..., l_{n-1}) that satisfy l_i < l_(i+1)
    """
    sequences = []
    
    # Generate all possible increasing sequences
    # We'll use a recursive approach to generate valid sequences
    def generate_recursive(current_seq, remaining_values):
        if len(current_seq) == n:
            sequences.append(tuple(current_seq))
            return
        
        # Add next value that's larger than the last one
        last_val = current_seq[-1] if current_seq else -1
        
        for val in range(last_val + 1, max_l_value + 1):
            generate_recursive(current_seq + [val], remaining_values)
    
    generate_recursive([], list(range(max_l_value + 1)))
    return sequences


def test_conjecture(n, l_values, verbose=True):
    """
    Test the conjecture for given n and l_values
    Returns (is_divisible, N, D, N_factors, D_factors, is_positive_ratio)
    """
    N = calculate_N(n, l_values)
    D = calculate_D(n, l_values[0])
    
    N_factors = prime_factors(N)
    D_factors = prime_factors(D)
    
    # Check if N/D is positive (conjecture only applies when positive)
    is_positive_ratio = (N * D > 0) if D != 0 else False
    
    # Check divisibility
    is_divisible = (N % D == 0) if D != 0 else False
    
    if verbose:
        print(f"\n{'='*60}")
        print(f"Testing n={n}, l_values={l_values}")
        print(f"{'='*60}")
        print(f"N = {N}")
        print(f"D = {D}")
        print(f"N factors: {format_prime_factors(N_factors)}")
        print(f"D factors: {format_prime_factors(D_factors)}")
        print(f"Is N/D positive? {is_positive_ratio}")
        print(f"Is N divisible by D? {is_divisible}")
        if is_divisible:
            print(f"N/D = {N // D}")
    
    return is_divisible, N, D, N_factors, D_factors, is_positive_ratio


def random_sample_test(max_n=300, max_l_value=600, samples_per_n=100):
    """
    Perform random sampling testing of the conjecture
    """
    print("Tim's Conjecture Random Sampling Test")
    print("="*50)
    print("Conjecture: N is NOT divisible by D")
    print("where N = 2*3^n + sum(3^(n-i)*2^(l_i)) - 2^(l_0)")
    print("and D = 2^(l_0+1) - 3^n")
    print("with l_i < l_(i+1) for all i")
    print("and N/D is positive")
    print("and n > 1")
    print("="*50)
    print(f"Testing n from 2 to {max_n}, sampling {samples_per_n} cases per n")
    print("="*50)
    
    counterexamples = []
    total_tests = 0
    positive_tests = 0
    
    # Set random seed for reproducibility
    random.seed(42)
    
    for n in range(2, max_n + 1):
        if n % 50 == 0 or n <= 10:  # Show progress every 50 n values
            print(f"\nTesting n = {n}")
        
        # For large n, we need larger l_0 values to get positive D
        # D = 2^(l_0+1) - 3^n, so we need 2^(l_0+1) > 3^n
        # This means l_0+1 > n*log_2(3) ≈ n*1.585
        min_l_0 = int(n * 1.585)
        max_l_0 = min(min_l_0 + 20, max_l_value)  # Sample in a reasonable range
        
        samples_tested = 0
        for _ in range(samples_per_n * 3):  # Try more samples to account for invalid ones
            if samples_tested >= samples_per_n:
                break
                
            # Generate random increasing sequence
            l_values = []
            current_val = random.randint(min_l_0, max_l_0)
            l_values.append(current_val)
            
            for i in range(1, n):
                # Next value must be larger than previous
                next_val = current_val + random.randint(1, min(10, max_l_value - current_val))
                l_values.append(next_val)
                current_val = next_val
            
            # Check if sequence is valid (strictly increasing)
            if not is_valid_l_sequence(l_values):
                continue
                
            total_tests += 1
            samples_tested += 1
            
            is_divisible, N, D, N_factors, D_factors, is_positive_ratio = test_conjecture(n, l_values, verbose=False)
            
            if is_positive_ratio:
                positive_tests += 1
                
                if is_divisible:
                    counterexamples.append((n, l_values, N, D))
                    print(f"\n🚨 COUNTEREXAMPLE FOUND! 🚨")
                    print(f"n={n}, l_values={l_values}")
                    print(f"N={N}, D={D}")
                    print(f"N factors: {format_prime_factors(N_factors)}")
                    print(f"D factors: {format_prime_factors(D_factors)}")
                    print(f"N/D = {N // D}")
                elif n <= 10:  # Only show details for small n
                    print(f"✓ n={n}, l={l_values} → N={N}, D={D} (not divisible)")
        
        # Show progress every 50 n values
        if n % 50 == 0:
            print(f"  Progress: n={n}, total_tests={total_tests}, positive_tests={positive_tests}, counterexamples={len(counterexamples)}")
    
    print(f"\n{'='*60}")
    print(f"FINAL SUMMARY")
    print(f"{'='*60}")
    print(f"Total tests: {total_tests}")
    print(f"Tests with positive N/D: {positive_tests}")
    print(f"Counterexamples found: {len(counterexamples)}")
    
    if counterexamples:
        print(f"\nCounterexamples (with positive N/D):")
        for n, l_values, N, D in counterexamples:
            print(f"  n={n}, l={l_values} → N={N}, D={D}")
    else:
        print(f"\n✅ Conjecture holds for all tested cases with positive N/D!")
    
    return counterexamples


def detailed_test_specific_cases():
    """
    Test some specific interesting cases in detail
    """
    print(f"\n{'='*60}")
    print("DETAILED TESTING OF SPECIFIC CASES")
    print(f"{'='*60}")
    
    # Test cases with small values (now increasing sequences)
    test_cases = [
        (1, [0]),         # n=1, l=[0]
        (1, [1]),         # n=1, l=[1]
        (2, [0, 1]),      # n=2, l=[0,1]
        (2, [0, 2]),      # n=2, l=[0,2]
        (3, [0, 1, 2]),   # n=3, l=[0,1,2]
    ]
    
    for n, l_values in test_cases:
        test_conjecture(n, l_values, verbose=True)


if __name__ == "__main__":
    # Run random sampling test
    counterexamples = random_sample_test(max_n=300, max_l_value=600, samples_per_n=50)
    
    # Run detailed tests
    detailed_test_specific_cases()
    
    print(f"\n{'='*60}")
    print("Analysis complete!")
    print(f"{'='*60}")
