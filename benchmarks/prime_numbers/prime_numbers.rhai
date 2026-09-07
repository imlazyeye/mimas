const MAX_NUMBER_TO_CHECK = 7000000;

let prime_mask = [];
for i in 0..(MAX_NUMBER_TO_CHECK + 1) {
    prime_mask.push(true);
}

prime_mask[0] = false;
prime_mask[1] = false;

let total_primes_found = 0;

let i = 2;
while i < MAX_NUMBER_TO_CHECK + 1 {
    if !prime_mask[i] {
        i += 1;
        continue;
    }

    total_primes_found += 1;

    let n = 2 * i;
    while n < MAX_NUMBER_TO_CHECK + 1 {
        prime_mask[n] = false;
        n += i;
    }

    i += 1;
}
print(total_primes_found);
