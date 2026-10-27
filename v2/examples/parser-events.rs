fn main() {
    let s = "    code\n\n- one\n    continued\n\nline\nnext\nline";
    for (e, r) in pulldown_cmark::Parser::new(s).into_offset_iter() {
        println!("{e:?} {r:?} {:?}", &s[r.clone()]);
    }
}
