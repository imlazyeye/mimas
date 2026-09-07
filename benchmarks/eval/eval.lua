local NUM = 0
local ADD = 1
local SUB = 2
local MUL = 3
local NEG = 4

local function eval(e)
    local t = e[1]
    if t == NUM then
        return e[2]
    elseif t == ADD then
        return eval(e[2]) + eval(e[3])
    elseif t == SUB then
        return eval(e[2]) - eval(e[3])
    elseif t == MUL then
        return eval(e[2]) * eval(e[3])
    else
        return -eval(e[2])
    end
end

local function build(depth)
    if depth <= 1 then
        return { NUM, depth + 1 }
    end
    local m = depth % 4
    if m == 0 then
        return { ADD, build(depth - 1), build(depth - 2) }
    elseif m == 1 then
        return { SUB, build(depth - 1), build(depth - 2) }
    elseif m == 2 then
        return { NEG, build(depth - 1) }
    else
        return { MUL, build(depth - 2), { NUM, 2 } }
    end
end

local DEPTH = 35
local REPEAT = 800

local tree = build(DEPTH)
local total = 0
for _ = 1, REPEAT do
    total = total + eval(tree)
end
print(total)
