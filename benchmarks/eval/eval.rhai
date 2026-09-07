fn ev(e) {
    if e.op == 0 { return e.val; }
    if e.op == 1 { return ev(e.l) + ev(e.r); }
    if e.op == 2 { return ev(e.l) - ev(e.r); }
    if e.op == 3 { return ev(e.l) * ev(e.r); }
    return -ev(e.l);
}

fn build(depth) {
    if depth <= 1 { return #{ op: 0, val: depth + 1 }; }
    let m = depth % 4;
    if m == 0 { return #{ op: 1, l: build(depth - 1), r: build(depth - 2) }; }
    if m == 1 { return #{ op: 2, l: build(depth - 1), r: build(depth - 2) }; }
    if m == 2 { return #{ op: 4, l: build(depth - 1) }; }
    return #{ op: 3, l: build(depth - 2), r: #{ op: 0, val: 2 } };
}

const DEPTH = 35;
const REPEAT = 800;

let tree = build(DEPTH);
let total = 0;
for i in 0..REPEAT {
    total += ev(tree);
}
print(total);
