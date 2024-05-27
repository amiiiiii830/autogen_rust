def sieve_of_eratosthenes(n):
    primes = [True for _ in range(n + 1)]
    p = 2
    while p**2 <= n:
        if primes[p] is True:
            for i in range(p**2, n + 1, p):
                primes[i] = False
        p += 1
    prime_numbers = [p for p in range(2, n) if primes[p]]
    return prime_numbers

print(sieve_of_eratosthenes(100))