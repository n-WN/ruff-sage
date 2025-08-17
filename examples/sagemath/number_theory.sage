# SageMath Number Theory Example
# Demonstrates number-theoretic functions unique to SageMath

# Prime number operations
p = 101
print(f"Is {p} prime? {is_prime(p)}")
print(f"Next prime after {p}: {next_prime(p)}")

# Factorization
n = 2^7 * 3^4 * 5^2
factors = factor(n)
print(f"Factorization of {n}: {factors}")

# Modular arithmetic
a = 17
m = 23
inv = inverse_mod(a, m)
print(f"Inverse of {a} modulo {m}: {inv}")
print(f"Verification: {a} * {inv} ≡ {(a * inv) % m} (mod {m})")

# Elliptic curves
E = EllipticCurve([0, 1])  # y^2 = x^3 + 1
print(f"Elliptic curve: {E}")

# Finite fields
F = GF(7)
print(f"Elements of GF(7): {list(F)}")

# Some code style issues for Ruff to detect
def badly_formatted_function( x,y ):  # Bad spacing
    return x^2+y^2  # Missing spaces around operators

result=badly_formatted_function(3,4)  # No space around =
print(result)