use crate::lex::{TokKind::*, TyKw};

tok_test!(whitepsace: " \n\t" =>);
tok_test!(int: "1" => Int(1));
tok_test!(int_with_underscores: "1_000_000" => Int(1_000_000));
tok_test!(int_with_irregular_underscores: "1_0_00___000" => Int(1_000_000));
tok_test!(float: "0.2" => Float(0.2));
tok_test!(float_zero: "0.0" => Float(0.0));
tok_test!(range: "0..1" => Int(0), DoubleDot, Int(1));
tok_test!(range_eq: "0..=1" => Int(0), DoubleDotEqual, Int(1));
tok_test!(access_int: "0.a" => Int(0), Dot, Ident("a"));
tok_test!(dot_vs_doubledot: ". .. ..=" => Dot, DoubleDot, DoubleDotEqual);
tok_test!(hex_0x: "0x1" => Hex("1"));
tok_test!(hex_upper: "0xFF" => Hex("FF"));
tok_test!(neg_int: "-1" => Minus, Int(1));
tok_test!(neg_float: "-1.5" => Minus, Float(1.5));

tok_test!(empty_string: "\"\"" => String(""));
tok_test!(string: "\"foo\"" => String("foo"));
tok_test!(multiline_string: "\"foo\nfoo\"" => String("foo\nfoo"));
tok_test!(escape_string: "\"foo\\\"bar\"" => String("foo\\\"bar"));
tok_test!(fstring_plain: "f\"\"" => FString(""));
tok_test!(fstring: "f\"foo {a}\"" => FString("foo {a}"));
tok_test!(fstring_with_quoted_index: "f\"{m[\"k\"]}\"" => FString("{m[\"k\"]}"));
tok_test!(fstring_with_quoted_brace: "f\"{m[\"}\"]}\"" => FString("{m[\"}\"]}"));
tok_test!(ident: "foo" => Ident("foo"));

tok_test!(tykw_int: "int" => TyKw(TyKw::Int));
tok_test!(tykw_float: "float" => TyKw(TyKw::Float));
tok_test!(tykw_str: "str" => TyKw(TyKw::Str));
tok_test!(tykw_bool: "bool" => TyKw(TyKw::Bool));

tok_test!(invalid_backtick: "`" => Invalid("`"));
tok_test!(invalid_hash: "#" => Invalid("#"));
tok_test!(invalid_dollar: "$" => Invalid("$"));
tok_test!(invalid_backslash: "\\" => Invalid("\\"));

tok_test!(line_comment_skipped: "1 // a comment\n2" => Int(1), Int(2));
tok_test!(comment_only: "// nothing here" =>);
tok_test!(comment_trailing_eof: "1 // tail" => Int(1));

tok_test!(block_comment_skipped: "1 /* a comment */ 2" => Int(1), Int(2));
tok_test!(block_comment_only: "/* nothing here */" =>);
tok_test!(block_comment_multiline: "1 /* a\nb\nc */ 2" => Int(1), Int(2));
tok_test!(block_comment_empty_body: "1 /**/ 2" => Int(1), Int(2));
tok_test!(block_comment_nested: "1 /* a /* b */ c */ 2" => Int(1), Int(2));
tok_test!(block_comment_shared_slash: "1 /*/ 2 */ 3" => Int(1), Int(3));
tok_test!(block_comment_holds_line_comment: "1 /* // not a line comment */ 2" => Int(1), Int(2));
tok_test!(block_comment_between_tokens: "a/*x*/+/*y*/b" => Ident("a"), Plus, Ident("b"));
tok_test!(block_comment_ignored_in_string: "\"/* not */\"" => String("/* not */"));
tok_test!(division_not_a_comment: "6 / 2" => Int(6), Slash, Int(2));

tok_test!(double_colon_path: "a::b" => Ident("a"), DoubleColon, Ident("b"));
tok_test!(tilde_dict: "~{}" => TildeLeftBrace, RightBrace);
tok_test!(tilde_slash: "a ~/ b" => Ident("a"), TildeSlash, Ident("b"));
tok_test!(hook_dot: "a?.b" => Ident("a"), HookDot, Ident("b"));
tok_test!(hook_left_square: "a?[0]" => Ident("a"), HookLeftSquare, Int(0), RightSquare);
tok_test!(double_hook_eq: "a ??= b" => Ident("a"), DoubleHookEqual, Ident("b"));
tok_test!(compound_assigns: "+= -= *= /= %= |= &= ^= ~/=" =>
    PlusEqual, MinusEqual, StarEqual, SlashEqual, PercentEqual,
    PipeEqual, AmpersandEqual, CaretEqual, TildeSlashEqual);
tok_test!(at_path: "@9" => At, Int(9));
tok_test!(keyword_ident_boundary: "ifx" => Ident("ifx"));
tok_test!(keyword_self: "self" => SelfKeyword);
