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
    """Calculate x for given k values using the new equation.
    
    General equation for n k values:
    - n=1: x = (2^1 - 2^{k_0-1}) / (2^{k_0} - 3^1)
    - n≥2: x = (2^(k_{n-1}+1)(2^(k_0+k_1+...+k_{n-2}+1) + 2^(k_1+k_2+...+k_{n-2}) + ... + 2^(k_{n-2}) + 1) - 2^n) / (2^(k_0+k_1+...+k_{n-1}) * (3^n - 2^(n-2)))
    
    This works for any number of k values: k_0, k_1, ..., k_{n-1}
    """
    n = len(k_values)
    if n < 1:
        return None
    
    # Calculate sum of all k values
    sum_k = sum(k_values)
    
    if n == 1:
        # Special case for n=1: x = (2^1 - 2^{k_0-1}) / (2^{k_0} - 3^1)
        numerator = 2**1 - 2**(k_values[0] - 1)
        denominator = 2**k_values[0] - 3**1
    else:
        # General case for n≥2: x = (2^(k_{n-1}+1)(2^(k_0+k_1+...+k_{n-2}+1) + 2^(k_1+k_2+...+k_{n-2}) + ... + 2^(k_{n-2}) + 1) - 2^n) / (2^(k_0+k_1+...+k_{n-1}) * (3^n - 2^(n-2)))
        
        # Calculate numerator: 2^(k_{n-1}+1)(2^(k_0+k_1+...+k_{n-2}+1) + 2^(k_1+k_2+...+k_{n-2}) + ... + 2^(k_{n-2}) + 1) - 2^n
        power_k_last_plus_1 = 2**(k_values[-1] + 1)
        
        # Build the nested sum inside the parentheses
        # Pattern: 2^(k_0+k_1+...+k_{n-2}+1) + 2^(k_1+k_2+...+k_{n-2}) + ... + 2^(k_{n-2}) + 1
        numerator_term = 0
        for i in range(n):
            if i == 0:
                # First term: 2^(k_0+k_1+...+k_{n-2}+1)
                sum_k_up_to_n_minus_2 = sum(k_values[:-1])  # Sum of k_0 to k_{n-2}
                numerator_term += 2**(sum_k_up_to_n_minus_2 + 1)
            elif i == n-1:
                # Last term: +1
                numerator_term += 1
            else:
                # Middle terms: 2^(k_i+k_{i+1}+...+k_{n-2})
                sum_k_from_i_to_n_minus_2 = sum(k_values[i:-1])
                numerator_term += 2**sum_k_from_i_to_n_minus_2
        
        numerator = power_k_last_plus_1 * numerator_term - 2**n
        
        # Calculate denominator: 2^(k_0+k_1+...+k_{n-1}) * (3^n - 2^(n-2))
        power_sum_k = 2**sum_k
        denominator_factor = 3**n - 2**(n-2)
        denominator = power_sum_k * denominator_factor
    
    # Calculate x as a fraction
    x_value = Fraction(numerator, denominator)
    
    return x_value, numerator, denominator, sum_k

def generate_k_permutations(n, min_avg=1.0, max_avg=2.0):
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
    """Generate table of k values and x values with prime factorization using the new equation.
    
    The new equation works for any number of k values: k_0, k_1, ..., k_{n-1}
    """
    generator_func = generate_k_permutations
    mode = "permutations"
    
    print(f"Generating {mode} for n=1 to {max_n}")
    print(f"New equation: x = (2^(k_{{n-1}}+1)(2^(k_0+k_1+...+k_{{n-2}}+1) + 2^(k_1+k_2+...+k_{{n-2}}) + ... + 2^(k_{{n-2}}) + 1) - 2^n) / (2^(k_0+k_1+...+k_{{n-1}}) * (3^n - 2^(n-2)))")
    print(f"{'n':<2} {'k values':<20} {'x value':<25} {'x (dec)':<10} {'Numerator':<15} {'Denominator':<15} {'Sum k':<8} {'Prime factors':<50}")
    print("-" * 180)

    total_count = 0
    integer_count = 0
    common_factors_found = False
    integer_solution_found = False

    for n in range(1, max_n + 1):
            
        for k_values in generator_func(n):
            result = calculate_cycle_x(k_values)
            if result is not None:
                x_frac, numerator, denominator, sum_k = result
                
                # Check for common factors between numerator and denominator
                # Only stop if there are common factors that don't result in an integer solution
                # Check the simplified fraction, not the raw numerator/denominator
                if has_common_factors(x_frac.numerator, x_frac.denominator) and x_frac.denominator != 1:
                    print(f"\n*** Common factors found between numerator and denominator! ***")
                    print(f"n={n}, k_values={k_values}")
                    print(f"x = {x_frac}")
                    print(f"Numerator: {x_frac.numerator}")
                    print(f"Denominator: {x_frac.denominator}")
                    common_factors_found = True
                
                total_count += 1
                x_decimal = float(x_frac)
                is_integer = x_frac.denominator == 1
                if is_integer:
                    integer_count += 1
                    # Only stop at positive non-zero integer solutions (skip negative and zero cases)
                    if x_frac.numerator > 0:
                        print(f"\n*** STOPPING: Positive integer solution found! ***")
                        print(f"n={n}, k_values={k_values}")
                        print(f"x = {x_frac} (positive integer)")
                        print(f"Numerator: {numerator}")
                        print(f"Denominator: {denominator}")
                        print(f"Sum k: {sum_k}")
                        integer_solution_found = True
                        break

                # Calculate prime factors for x
                num_factors = prime_factors(abs(x_frac.numerator))
                den_factors = prime_factors(abs(x_frac.denominator))
                
                # Format prime factors
                num_factors_str = format_prime_factors(num_factors)
                den_factors_str = format_prime_factors(den_factors)
                
                prime_factors_str = f"Num: {num_factors_str} | Den: {den_factors_str}"

                # Format the output
                k_str = str(k_values)
                x_frac_str = str(x_frac)
                numerator_str = str(numerator)
                denominator_str = str(denominator)
                sum_k_str = str(sum_k)

                # Mark integers with *
                marker = " *" if is_integer else ""
                
                # Check if numerator >= denominator for highlighting (using prime factorization)
                # Get the actual values from prime factorization
                if numerator == 0:
                    num_value = 0
                else:
                    num_value = 1
                    for prime, power in num_factors.items():
                        if prime != 1:
                            num_value *= prime ** power
                
                den_value = 1
                for prime, power in den_factors.items():
                    if prime != 1:
                        den_value *= prime ** power
                
                numerator_greater_equal = abs(num_value) >= abs(den_value)
                
                # Also highlight integer solutions
                is_integer = x_frac.denominator == 1
                
                # Create the output line
                output_line = f"{n:<2} {k_str:<20} {x_frac_str:<25} {x_decimal:<10.6f}{marker} {numerator_str:<15} {denominator_str:<15} {sum_k_str:<8} {prime_factors_str}"
                
                # Highlight in green if numerator >= denominator OR if it's an integer solution
                if numerator_greater_equal or is_integer:
                    print(f"\033[92m{output_line}\033[0m")  # Green color
                else:
                    print(output_line)

    print("-" * 180)
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
    # Generate table for various n values using the generalized equation
    generate_table(max_n=6)
