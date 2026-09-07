local MAX = 7000000

local prime_mask = {}
for i = 0, MAX do
    prime_mask[i] = true
end

prime_mask[0] = false
prime_mask[1] = false

local total = 0

local i = 2
while i < MAX + 1 do
    if not prime_mask[i] then
        i = i + 1
    else
        total = total + 1
        local n = 2 * i
        while n < MAX + 1 do
            prime_mask[n] = false
            n = n + i
        end
        i = i + 1
    end
end
print(total)
