# SageMath Basic Examples
# This file demonstrates SageMath syntax that differs from Python

# 1. Power operator using ^ instead of **
x = 2^3           # SageMath: 2^3, Python: 2**3
y = 5^2^3         # Nested powers
print(f"2^3 = {x}")
print(f"5^2^3 = {y}")

# 2. Rational number literals
r1 = 1/3          # Exact rational in SageMath
r2 = 22/7         # Another rational
print(f"1/3 = {r1}")
print(f"22/7 = {r2}")

# 3. Integer ring operations
a = 17
b = 12
gcd_val = gcd(a, b)
print(f"gcd({a}, {b}) = {gcd_val}")

# 4. Symbolic variables (SageMath specific)
var('t')
f = t^2 + 3*t + 2
print(f"f(t) = {f}")

# 5. Matrix operations
M = Matrix([[1, 2], [3, 4]])
print(f"Matrix M = {M}")
print(f"Determinant = {M.determinant()}")

# 6. Some potential style issues for Ruff to catch
unused_variable = 42  # Unused variable
print( "hello"  )     # Extra spaces
import os, sys        # Multiple imports on one line