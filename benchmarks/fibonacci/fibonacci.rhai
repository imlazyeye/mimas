const TARGET = 28;
const REPEAT = 30;

fn fib(n) {
    if n < 2 { return n; }
    return fib(n - 1) + fib(n - 2);
}

let result = 0;
for n in 0..REPEAT {
    result = fib(TARGET);
}
print(result);
