local WIDTH = 450
local HEIGHT = 450
local MAXITER = 400

local checksum = 0
for py = 0, HEIGHT - 1 do
    local cy = (py / HEIGHT) * 3.0 - 1.5
    for px = 0, WIDTH - 1 do
        local cx = (px / WIDTH) * 3.0 - 2.0
        local zx = 0.0
        local zy = 0.0
        local iter = 0
        while (zx * zx + zy * zy <= 4.0) and (iter < MAXITER) do
            local new_zx = (zx * zx - zy * zy) + cx
            zy = (2.0 * zx * zy) + cy
            zx = new_zx
            iter = iter + 1
        end
        checksum = checksum + iter
    end
end
print(checksum)
