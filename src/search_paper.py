import io
import sys
output = io.StringIO()
old_output = sys.stdout
sys.stdout = output
def is_prime(n):
    if n <= 1:
        return False
    if n == 2:
        return True
    if n % 2 == 0:
        return False
    i = 3
    while i * i <= n:
        if n % i == 0:
            return False
        i += 2
    return True

prime_numbers_below_100 = [num for num in range(2, 100) if is_prime(num)]

print(prime_numbers_below_100)

captured_output = output.getvalue()
with open("save.txt", 'w') as file:
    file.write(captured_output)
sys.stdout = old_output
print(captured_output)