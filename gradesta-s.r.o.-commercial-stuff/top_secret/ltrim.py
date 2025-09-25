def int_to_base(n: int, base: int, *, keep_sign: bool = True) -> str:
    """
    Convert *any* signed integer to a base-`base` string (2 ≤ base ≤ 36).

    - If keep_sign is True, a leading '-' is added for negative n.
    - If keep_sign is False, the sign is ignored (absolute value is used).
    """
    if not (2 <= base <= 36):
        raise ValueError("base must be in the range 2–36")
    # Special-case zero
    if n == 0:
        return "0"
    digits = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"
    sign = "-" if (n < 0 and keep_sign) else ""
    n = abs(n)
    out = []
    while n:
        n, r = divmod(n, base)
        out.append(digits[r])
    return sign + "".join(reversed(out))



def ltrim(number: int, base: int, digits: int) -> int:
    """
    1. Keep only the last `digits` base-`base` digits of `number`.
    2. Print those digits as a string in that base.
    3. Return the numeric value represented by those digits.

    Examples
    --------
    ltrim(123456, 10, 3)      # prints 456   -> returns 456
    ltrim(255,     16, 2)     # prints FF    -> returns 255
    ltrim(10,      2, 4)      # prints 1010  -> returns 10
    ltrim(-98765,  36, 4)     # prints -1U1T -> returns -8733
    """
    if not (2 <= base <= 36):
        raise ValueError("base must be between 2 and 36 (inclusive).")
    if digits < 1:
        raise ValueError("digits must be a positive integer.")
    cutoff = base ** digits
    abs_rem = abs(number) % cutoff            # numeric remainder
    str_rem = int_to_base(abs_rem, base).zfill(digits)
    print(str_rem)
    # Return signed numeric value
    return abs_rem
