const WIDTH = 450, HEIGHT = 450, MAXITER = 400;

let checksum = 0;
for (let py = 0; py < HEIGHT; py++) {
    const cy = (py / HEIGHT) * 3.0 - 1.5;
    for (let px = 0; px < WIDTH; px++) {
        const cx = (px / WIDTH) * 3.0 - 2.0;
        let zx = 0.0, zy = 0.0, iter = 0;
        while (zx * zx + zy * zy <= 4.0 && iter < MAXITER) {
            const new_zx = (zx * zx - zy * zy) + cx;
            zy = (2.0 * zx * zy) + cy;
            zx = new_zx;
            iter = iter + 1;
        }
        checksum = checksum + iter;
    }
}
console.log(checksum);
