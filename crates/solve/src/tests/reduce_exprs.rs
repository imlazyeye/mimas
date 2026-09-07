use parse::Literal::*;

// Basic literals
test_reduction!(int, "1" => Int(1));
test_reduction!(hex, "0xf" => Int(15));
test_reduction!(float, "1.2" => Float(1.2));
test_reduction!(string, "\"foo\"" => String("foo".into()));
test_reduction!(r#true, "true" => True);
test_reduction!(r#false, "false" => False);
test_reduction!(null, "null" => Null);
test_reduction!(unit, "()" => Unit);

// Logicals
test_reduction!(
    and,
    "true && true" => True,
    "true && false" => False,
    "false && false" => False,
);
test_reduction!(
    or,
    "true || true" => True,
    "true || false" => True,
    "false || false" => False,
);

// Equalities
test_reduction!(
    equal,
    "() == ()" => True,
    "null == null" => True,
    "true == true" => True,
    "true == false" => False,
    "false == false" => True,
    "false == true" => False,
    "0 == 0" => True,
    "0 == 1" => False,
    "0.0 == 0.0" => True,
    "0.0 == 0.1" => False,
    "0 == 0.0" => True,
    "0 == 0.1" => False,
    r#""foo" == "foo""# => True,
    r#""foo" == "bar""# => False,
);
test_reduction!(
    not_equal,
    "() != ()" => False,
    "null != null" => False,
    "true != true" => False,
    "true != false" => True,
    "false != false" => False,
    "false != true" => True,
    "0 != 0" => False,
    "0 != 1" => True,
    "0.0 != 0.0" => False,
    "0.0 != 0.1" => True,
    "0 != 0.0" => False,
    "0 != 0.1" => True,
    r#""foo" != "foo""# => False,
    r#""foo" != "bar""# => True,
);
test_reduction!(
    greater_than,
    "0 > 1" => False,
    "1 > 1" => False,
    "2 > 1" => True,
    "0.0 > 1.0" => False,
    "1.0 > 1.0" => False,
    "2.0 > 1.0" => True,
    "0 > 1.0" => False,
    "1 > 1.0" => False,
    "2 > 1.0" => True,
);
test_reduction!(
    greater_than_or_equal,
    "0 >= 1" => False,
    "1 >= 1" => True,
    "2 >= 1" => True,
    "0.0 >= 1.0" => False,
    "1.0 >= 1.0" => True,
    "2.0 >= 1.0" => True,
    "0 >= 1.0" => False,
    "1 >= 1.0" => True,
    "2 >= 1.0" => True,
);
test_reduction!(
    less_than,
    "0 < 1" => True,
    "1 < 1" => False,
    "2 < 1" => False,
    "0.0 < 1.0" => True,
    "1.0 < 1.0" => False,
    "2.0 < 1.0" => False,
    "0 < 1.0" => True,
    "1 < 1.0" => False,
    "2 < 1.0" => False,
);
test_reduction!(
    less_than_or_equal,
    "0 <= 1" => True,
    "1 <= 1" => True,
    "2 <= 1" => False,
    "0.0 <= 1.0" => True,
    "1.0 <= 1.0" => True,
    "2.0 <= 1.0" => False,
    "0 <= 1.0" => True,
    "1 <= 1.0" => True,
    "2 <= 1.0" => False,
);
test_reduction!(
    positive,
    "+1" => Int(1),
    "+-1" => Int(1),
    "+1.2" => Float(1.2),
    "+-1.2" => Float(1.2),
);
test_reduction!(
    negative,
    "-1" => Int(-1),
    "--1" => Int(1),
    "-1.2" => Float(-1.2),
    "--1.2" => Float(1.2),
);
test_reduction!(
    not,
    "!true" => False,
    "!false" => True,
);
test_reduction!(
    bitwise_not,
    "~0" => Int(-1),
    "~1" => Int(-2),
);
test_reduction!(group, "(1)" => Int(1));

// Arithmetic
test_reduction!(
    addition,
    "1 + 1" => Int(2),
    "1.5 + 0.5" => Float(2.0),
    "1 + 0.5" => Float(1.5),
);
test_reduction!(
    subtraction,
    "5 - 3" => Int(2),
    "1.5 - 0.5" => Float(1.0),
);
test_reduction!(
    multiplication,
    "2 * 3" => Int(6),
    "2.0 * 0.5" => Float(1.0),
);
test_reduction!(
    division,
    "10.0 / 4.0" => Float(2.5),
    "10 / 4" => Float(2.5),
);
test_reduction!(integer_div, "10 ~/ 3" => Int(3));
test_reduction!(modulo, "5 % 2" => Int(1));

// Bitwise
test_reduction!(
    bitwise,
    "0xff & 0x0f" => Int(0x0f),
    "0x0f | 0xf0" => Int(0xff),
    "0xff ^ 0xff" => Int(0),
    "1 << 3" => Int(8),
    "8 >> 3" => Int(1),
);

// Cannot be reduced
test_reduction!(access, "foo.bar" => None);
test_reduction!(block_with_stmts, "{ let a = 0; 0 }" => None);
test_reduction!(r#break, "break 0" => None);
test_reduction!(call, "foo()" => None);
test_reduction!(collect, "collect 0" => None);
test_reduction!(r#continue, "continue" => None);
test_reduction!(r#for, "for a in b {}" => None);
test_reduction!(ident, "foo" => None);
test_reduction!(r#loop, "loop {}" => None);
test_reduction!(r#match, "match foo {}" => None);
test_reduction!(coalescence, "foo ?? bar" => None);
test_reduction!(r#return, "return 0" => None);
test_reduction!(unwrap, "a!" => None);
test_reduction!(r#while, "while foo {}" => None);
