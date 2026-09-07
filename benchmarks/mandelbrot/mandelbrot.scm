(define WIDTH 450)
(define HEIGHT 450)
(define MAXITER 400)

(define checksum
    (let row ((py 0) (acc 0))
        (if (< py HEIGHT)
            (let ((cy (- (* (/ (exact->inexact py) (exact->inexact HEIGHT)) 3.0) 1.5)))
                (row (+ py 1)
                    (let col ((px 0) (acc acc))
                        (if (< px WIDTH)
                            (let ((cx (- (* (/ (exact->inexact px) (exact->inexact WIDTH)) 3.0) 2.0)))
                                (col (+ px 1)
                                    (+ acc
                                        (let escape ((zx 0.0) (zy 0.0) (iter 0))
                                            (if (and (<= (+ (* zx zx) (* zy zy)) 4.0) (< iter MAXITER))
                                                (let ((new-zx (+ (- (* zx zx) (* zy zy)) cx)))
                                                    (escape new-zx
                                                        (+ (* (* 2.0 zx) zy) cy)
                                                        (+ iter 1)))
                                                iter)))))
                            acc))))
            acc)))

(displayln checksum)
