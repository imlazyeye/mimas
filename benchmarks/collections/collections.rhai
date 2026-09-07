const N = 4000000;
const K = 256;

let keys = [];
for j in 0..K {
    keys.push("k" + j);
}

let data = [];
for i in 0..N {
    data.push(i % K);
}

let hist = #{};
for x in data {
    let key = keys[x];
    if key in hist {
        hist[key] = hist[key] + 1;
    } else {
        hist[key] = 1;
    }
}

let total = 0;
for v in hist.values() {
    total += v;
}
print(total);
