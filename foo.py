def nthFibonacci(n):
    if n <= 1:
        return n

    # stores current Fibonacci number
    curr = 0

    # To store the previous 
    # two Fibonacci numbers
    prev1 = 1
    prev2 = 0

    for i in range(2, n + 1):
        curr = prev1 + prev2

        # Update previous two Fibonacci 
        # numbers for next number
        prev2 = prev1
        prev1 = curr

    return curr

if __name__ == "__main__":
    n = 500
    result = nthFibonacci(n)
    print(result)
