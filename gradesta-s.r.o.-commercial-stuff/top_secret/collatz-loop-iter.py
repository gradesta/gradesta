import itertools
from fractions import Fraction
from math import ceil
from collections import Counter

def prime_factors(n):
    """Return prime factors of n as a Counter object."""
    if n < 1:
        return Counter()
    if n == 1:
        return Counter({1: 1})
    
    factors = Counter()
    d = 2
    while d * d <= n:
        while n % d == 0:
            factors[d] += 1
            n //= d
        d += 1
    if n > 1:
        factors[n] += 1
    return factors

def format_prime_factors(factors):
    """Format prime factors as a string like '2^3 * 3^2 * 5'."""
    if not factors:
        return "1"
    
    parts = []
    for prime, exp in sorted(factors.items()):
        if exp == 1:
            parts.append(str(prime))
        else:
            parts.append(f"{prime}^{exp}")
    return " * ".join(parts)

def has_common_factors(num, den):
    """Check if numerator and denominator have any common prime factors."""
    if num == 0 or den == 0:
        return False
    
    num_factors = prime_factors(abs(num))
    den_factors = prime_factors(abs(den))
    
    # Check for any common prime factors (excluding 1)
    for prime in num_factors:
        if prime != 1 and prime in den_factors:
            return True
    return False

def calculate_cycle_x(k_values):
    """Calculate x for given k values."""
    n = len(k_values)
    rise = 3**n
    run = 2**(sum(k_values))

    # Calculate C (constant term)
    C = Fraction(0)
    for level in range(n):
        power_of_3 = 3**(n - 1 - level)
        k_sum_from_here = sum(k_values[level:])
        numerator = power_of_3 * (4 - 2**k_values[level])
        denominator = 2 * (2**k_sum_from_here)
        C += Fraction(numerator, denominator)

    C -= Fraction(1, 2)

    # Calculate x = (C * run) / (run - rise)
    x_value = Fraction(C * run, run - rise)

    return x_value, C, rise, run

def generate_k_permutations(n, min_avg=1.585, max_avg=1.737):
    """Generate all valid k permutations for cycle length n."""
    min_sum = max(n, ceil(n * min_avg))
    max_sum = int(n * max_avg)
    max_individual_k = int(n * max_avg - n + 1)

    for total_sum in range(min_sum, max_sum + 1):
        avg = total_sum / n
        if not (min_avg <= avg <= max_avg):
            continue

        def generate_partitions(remaining_sum, remaining_slots, min_val=1):
            if remaining_slots == 1:
                if min_val <= remaining_sum <= max_individual_k:
                    yield [remaining_sum]
                return

            max_val_here = min(max_individual_k, remaining_sum - remaining_slots + 1)
            for val in range(min_val, max_val_here + 1):
                for partition in generate_partitions(remaining_sum - val, remaining_slots - 1, val):
                    yield [val] + partition

        # Generate all unique combinations first
        combinations = set()
        for partition in generate_partitions(total_sum, n):
            # Convert to tuple for hashing and to ensure uniqueness
            combinations.add(tuple(partition))
        
        # Generate all unique permutations of each unique combination
        for combination in combinations:
            # Use a set to deduplicate permutations when there are duplicate values
            unique_perms = set()
            for perm in itertools.permutations(combination):
                unique_perms.add(perm)
            
            # Yield each unique permutation
            for perm in unique_perms:
                yield list(perm)

def generate_table(max_n=6):
    """Generate table of n, k values, and x values with prime factorization."""
    generator_func = generate_k_permutations
    mode = "permutations"
    
    print(f"Generating {mode} for n=1 to {max_n}")
    print(f"{'n':<2} {'k values':<20} {'x value':<25} {'x (dec)':<10} {'C':<20} {'rise':<8} {'run':<12} {'Prime factors':<50}")
    print("-" * 160)

    total_count = 0
    integer_count = 0
    common_factors_found = False
    integer_solution_found = False

    for n in range(1, max_n + 1):
        if common_factors_found or integer_solution_found:
            break
            
        for k_values in generator_func(n):
            result = calculate_cycle_x(k_values)
            if result is not None:
                x_frac, C, rise, run = result
                
                # Check for common factors between numerator and denominator
                if has_common_factors(x_frac.numerator, x_frac.denominator):
                    print(f"\n*** STOPPING: Common factors found between numerator and denominator! ***")
                    print(f"n={n}, k_values={k_values}")
                    print(f"x = {x_frac}")
                    print(f"Numerator: {x_frac.numerator}")
                    print(f"Denominator: {x_frac.denominator}")
                    common_factors_found = True
                    break
                
                total_count += 1
                x_decimal = float(x_frac)
                is_integer = x_frac.denominator == 1
                if is_integer:
                    integer_count += 1
                    # Only stop at non-zero integer solutions (skip the trivial x=0 case)
                    if x_frac.numerator != 0:
                        print(f"\n*** STOPPING: Non-zero integer solution found! ***")
                        print(f"n={n}, k_values={k_values}")
                        print(f"x = {x_frac} (integer)")
                        print(f"C = {C}")
                        print(f"rise = {rise}, run = {run}")
                        integer_solution_found = True
                        break

                # Calculate prime factors for x
                num_factors = prime_factors(abs(x_frac.numerator))
                den_factors = prime_factors(abs(x_frac.denominator))
                
                # Format prime factors
                num_factors_str = format_prime_factors(num_factors)
                den_factors_str = format_prime_factors(den_factors)
                
                if x_frac.denominator == 1:
                    prime_factors_str = f"Num: {num_factors_str}"
                else:
                    prime_factors_str = f"Num: {num_factors_str} | Den: {den_factors_str}"

                # Format the output
                k_str = str(k_values)
                x_frac_str = str(x_frac)
                C_str = str(C)
                rise_str = str(rise)
                run_str = str(run)

                # Mark integers with *
                marker = " *" if is_integer else ""

                print(f"{n:<2} {k_str:<20} {x_frac_str:<25} {x_decimal:<10.6f}{marker} {C_str:<20} {rise_str:<8} {run_str:<12} {prime_factors_str}")

    print("-" * 160)
    if common_factors_found:
        print(f"Search stopped due to common factors found.")
        print(f"Solutions checked before stopping: {total_count}")
    elif integer_solution_found:
        print(f"Search stopped due to non-zero integer solution found.")
        print(f"Solutions checked before stopping: {total_count}")
    else:
        print(f"Total solutions: {total_count}")
    print(f"Integer solutions: {integer_count}")
    print("* indicates integer solutions (potential cycles)")

if __name__ == "__main__":
    # Set to True to generate all permutations, False for combinations only
    generate_table(max_n=10)
