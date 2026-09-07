var COUNT = 250;
var STEPS = 600;
var SIZE = 400.0;
var TOUCH = 8.0;

var balls = array_create(COUNT);
for (var i = 0; i < COUNT; i++) {
    balls[i] = { x: (i mod 25) * 16.0, y: (i mod 17) * 23.0, vx: (i mod 7) - 3.0, vy: (i mod 5) - 2.0 };
}

repeat STEPS {
    // move each ball, reversing it whenever it reaches a wall
    for (var i = 0; i < COUNT; i++) {
        var ball = balls[i];
        ball.x += ball.vx;
        ball.y += ball.vy;
        if ball.x < 0.0 || ball.x > SIZE { ball.vx = -ball.vx; }
        if ball.y < 0.0 || ball.y > SIZE { ball.vy = -ball.vy; }
    }
    // when two balls touch, swap their velocities so they bounce apart
    for (var i = 0; i < COUNT; i++) {
        var ball = balls[i];
        for (var j = i + 1; j < COUNT; j++) {
            var mate = balls[j];
            var dx = mate.x - ball.x;
            var dy = mate.y - ball.y;
            if sqrt(dx * dx + dy * dy) < TOUCH {
                var tx = ball.vx; var ty = ball.vy;
                ball.vx = mate.vx; ball.vy = mate.vy;
                mate.vx = tx; mate.vy = ty;
            }
        }
    }
}

var total = 0.0;
for (var i = 0; i < COUNT; i++) {
    total += balls[i].x + balls[i].y;
}
show_debug_message(string(total));
