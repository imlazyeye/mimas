const COUNT = 250;
const STEPS = 600;
const SIZE = 400.0;
const TOUCH = 8.0;

let balls = [];
for i in 0..COUNT {
    balls.push(#{ x: (i % 25).to_float() * 16.0, y: (i % 17).to_float() * 23.0, vx: (i % 7).to_float() - 3.0, vy: (i % 5).to_float() - 2.0 });
}

for step in 0..STEPS {
    // move each ball, reversing it whenever it reaches a wall
    for i in 0..COUNT {
        balls[i].x += balls[i].vx;
        balls[i].y += balls[i].vy;
        if balls[i].x < 0.0 || balls[i].x > SIZE { balls[i].vx = -balls[i].vx; }
        if balls[i].y < 0.0 || balls[i].y > SIZE { balls[i].vy = -balls[i].vy; }
    }
    // when two balls touch, swap their velocities so they bounce apart
    for i in 0..COUNT {
        for j in (i + 1)..COUNT {
            let dx = balls[j].x - balls[i].x;
            let dy = balls[j].y - balls[i].y;
            if (dx * dx + dy * dy).sqrt() < TOUCH {
                let tx = balls[i].vx;
                let ty = balls[i].vy;
                balls[i].vx = balls[j].vx;
                balls[i].vy = balls[j].vy;
                balls[j].vx = tx;
                balls[j].vy = ty;
            }
        }
    }
}

let total = 0.0;
for i in 0..COUNT {
    total += balls[i].x + balls[i].y;
}
print(total);
