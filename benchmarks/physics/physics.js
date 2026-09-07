const COUNT = 250, STEPS = 600, SIZE = 400.0, TOUCH = 8.0;

const balls = [];
for (let i = 0; i < COUNT; i++) {
    balls.push({ x: (i % 25) * 16.0, y: (i % 17) * 23.0, vx: (i % 7) - 3.0, vy: (i % 5) - 2.0 });
}

for (let s = 0; s < STEPS; s++) {
    // move each ball, reversing it whenever it reaches a wall
    for (const ball of balls) {
        ball.x += ball.vx;
        ball.y += ball.vy;
        if (ball.x < 0.0 || ball.x > SIZE) ball.vx = -ball.vx;
        if (ball.y < 0.0 || ball.y > SIZE) ball.vy = -ball.vy;
    }
    // when two balls touch, swap their velocities so they bounce apart
    for (let i = 0; i < COUNT; i++) {
        const ball = balls[i];
        for (let j = i + 1; j < COUNT; j++) {
            const other = balls[j];
            const dx = other.x - ball.x;
            const dy = other.y - ball.y;
            if (Math.sqrt(dx * dx + dy * dy) < TOUCH) {
                [ball.vx, ball.vy, other.vx, other.vy] = [other.vx, other.vy, ball.vx, ball.vy];
            }
        }
    }
}

let total = 0.0;
for (const ball of balls) {
    total += ball.x + ball.y;
}
console.log(total);
