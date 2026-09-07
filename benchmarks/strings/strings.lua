local N = 4500000

local total_len = 0
local hits = 0
for i = 0, N - 1 do
    local s = "item_" .. (i % 128) .. "_value"
    total_len = total_len + #s
    if string.find(s, "7", 1, true) ~= nil then
        hits = hits + 1
    end
    local u = string.upper(s)
    total_len = total_len + #u
end
print(total_len)
