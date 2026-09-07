local COUNT = 250
local STEPS = 600
local SIZE = 400.0
local TOUCH = 8.0

local balls = {}
for i = 0, COUNT - 1 do
    balls[i + 1] = { x = (i % 25) * 16.0, y = (i % 17) * 23.0, vx = (i % 7) - 3.0, vy = (i % 5) - 2.0 }
end

for _ = 1, STEPS do
    -- move each ball, reversing it whenever it reaches a wall
    for i = 1, COUNT do
        local ball = balls[i]
        ball.x = ball.x + ball.vx
        ball.y = ball.y + ball.vy
        if ball.x < 0.0 or ball.x > SIZE then ball.vx = -ball.vx end
        if ball.y < 0.0 or ball.y > SIZE then ball.vy = -ball.vy end
    end
    -- when two balls touch, swap their velocities so they bounce apart
    for i = 1, COUNT do
        local ball = balls[i]
        for j = i + 1, COUNT do
            local other = balls[j]
            local dx = other.x - ball.x
            local dy = other.y - ball.y
            if math.sqrt(dx * dx + dy * dy) < TOUCH then
                ball.vx, ball.vy, other.vx, other.vy = other.vx, other.vy, ball.vx, ball.vy
            end
        end
    end
end

local total = 0.0
for i = 1, COUNT do
    total = total + balls[i].x + balls[i].y
end
print(total)
