import math


class Ball:
    __slots__ = ("x", "y", "vx", "vy")

    def __init__(self, x, y, vx, vy):
        self.x = x
        self.y = y
        self.vx = vx
        self.vy = vy


def main():
    COUNT = 250
    STEPS = 600
    SIZE = 400.0
    TOUCH = 8.0

    balls = []
    for i in range(COUNT):
        balls.append(Ball(float(i % 25) * 16.0, float(i % 17) * 23.0, float(i % 7) - 3.0, float(i % 5) - 2.0))

    for _ in range(STEPS):
        # move each ball, reversing it whenever it reaches a wall
        for ball in balls:
            ball.x += ball.vx
            ball.y += ball.vy
            if ball.x < 0.0 or ball.x > SIZE:
                ball.vx = -ball.vx
            if ball.y < 0.0 or ball.y > SIZE:
                ball.vy = -ball.vy
        # when two balls touch, swap their velocities so they bounce apart
        for i in range(COUNT):
            ball = balls[i]
            for j in range(i + 1, COUNT):
                other = balls[j]
                dx = other.x - ball.x
                dy = other.y - ball.y
                if math.sqrt(dx * dx + dy * dy) < TOUCH:
                    ball.vx, ball.vy, other.vx, other.vy = other.vx, other.vy, ball.vx, ball.vy

    total = 0.0
    for ball in balls:
        total += ball.x + ball.y
    print(total)


main()
