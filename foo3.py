import math
import random

def foo(a, b):
    print("=====GCD=====")
    print(a)
    print(b)
    print(math.gcd(a, b))

foo(1, 2)
foo(0, 2)
foo(1111111111, 11)
foo(11 ** 10 * 2 ** 8 * 3 ** 9, 3 ** 2 * 2 ** 4)

for _ in range(0, 4):
    foo(random.randint(2**128, 2**1028), random.randint(2**128, 2**512))