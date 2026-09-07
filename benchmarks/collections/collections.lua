local N = 4000000
local K = 256

local keys = {}
for j = 0, K - 1 do
    keys[j + 1] = "k" .. j
end

local data = {}
for i = 0, N - 1 do
    data[i + 1] = i % K
end

local hist = {}
for _, x in ipairs(data) do
    local key = keys[x + 1]
    if hist[key] ~= nil then
        hist[key] = hist[key] + 1
    else
        hist[key] = 1
    end
end

local total = 0
for _, v in pairs(hist) do
    total = total + v
end
print(total)
