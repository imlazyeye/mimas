local TARGET = 28
local REPEAT = 30

local function fib(n)
    if n < 2 then return n end
    return fib(n - 1) + fib(n - 2)
end

local result = 0
for _ = 1, REPEAT do
    result = fib(TARGET)
end
print(result)
