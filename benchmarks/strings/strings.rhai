const N = 4500000;

let total_len = 0;
let hits = 0;
for i in 0..N {
    let s = "item_" + (i % 128) + "_value";
    total_len += s.len;
    if s.contains("7") {
        hits += 1;
    }
    let u = s.to_upper();
    total_len += u.len;
}
print(total_len);
