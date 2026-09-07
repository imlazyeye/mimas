#[macro_export]
macro_rules! ident_expr {
    ($name:expr) => {{
        use $crate::IntoExpr;
        $crate::expr::Ident::synthetic($name).into_expr()
    }};
}

#[macro_export]
macro_rules! ident {
    ($name:expr) => {
        $crate::expr::Ident::synthetic($name)
    };
}

#[macro_export]
macro_rules! int {
    ($num:expr) => {{
        use $crate::IntoExpr;
        $crate::expr::Literal::Int($num).into_expr()
    }};
}

#[macro_export]
macro_rules! float {
    ($num:expr) => {{
        use $crate::IntoExpr;
        $crate::expr::Literal::Float($num).into_expr()
    }};
}

#[macro_export]
macro_rules! block {
    () => {
        {
            use $crate::IntoExpr;
            #[allow(unused_mut)]
            let mut stmts = vec![];
            $crate::expr::Block::new(stmts).into_expr()
        }
    };

    ($($stmt:expr), * $(,)?) => {
        {
            use $crate::IntoExpr;
            #[allow(unused_mut)]
            let mut stmts = vec![$(
                $stmt
            )*];
            $crate::expr::Block::new(stmts).into_expr()
        }
    };
}

#[macro_export]
macro_rules! struc {
    ($name:ident) => {{
        #[allow(unused_mut)]
        $crate::item::Struct::new(
            $crate::expr::Ident::synthetic(stringify!($name).to_string()).into(),
            vec![],
        )
    }};

    ($name:ident { $($field: ident: $ty: expr),* }) => {
        {
            #[allow(unused_mut)]
            let fields = vec![
                $($crate::item::StructField {
                        name: $crate::item::FieldKey::Ident($crate::expr::Ident::synthetic(stringify!($field).to_string())),
                        annotation: $ty,
                        location: shared::Location::default(),
                        public: false,
                }),*
            ];
            $crate::item::Struct::new(
                $crate::expr::Ident::synthetic(stringify!($name).to_string()).into(),
                fields,
            )
        }
    };

    ($name:ident( $($kw:expr),*$(,)? )) => {
         {
            #[allow(unused_mut)]
            let mut fields = vec![];
            let mut iter = 0;
            $(
                let field = $crate::item::StructField {
                    name: $crate::item::FieldKey::Int(iter),
                    annotation: $kw,
                    location: shared::Location::default(),
                    public: false,
                };
                #[allow(unused_assignments)]
                {
                    iter += 1;
                }
                fields.push(field);
            )*
            $crate::item::Struct::new(
                $crate::expr::Ident::synthetic(stringify!($name).to_string()).into(),
                fields,
            )
        }
    }
}

#[macro_export]
macro_rules! enu {
    ($name:ident { $($member:expr),* }) => {
        {
            #[allow(unused_mut)]
            let members = vec![
                $($member),*
            ];
            $crate::item::Enum::new(
                $crate::expr::Ident::synthetic(stringify!($name).to_string()).into(),
                members,
            )
        }
    };
}

#[macro_export]
macro_rules! enum_struct {
     ($name:ident) => {{
        #[allow(unused_mut)]
         (
            $crate::expr::Ident::synthetic(stringify!($name).to_string()),
            $crate::item::Member::Struct(vec![])
        )
    }};

    ($name:ident { $($field: ident: $ty: expr),* }) => {
        {
            #[allow(unused_mut)]
            let fields = vec![
                $($crate::item::StructField {
                    name: $crate::item::FieldKey::Ident($crate::expr::Ident::synthetic(stringify!($field).to_string())),
                    annotation: $ty,
                    location: shared::Location::default(),
                    public: false,
                }),*
            ];
            (
                $crate::expr::Ident::synthetic(stringify!($name).to_string()),
                $crate::item::Member::Struct(fields)
            )

        }
    };
}

#[macro_export]
macro_rules! enum_tuple {
    ($name:ident()) => {{
        (
            $crate::expr::Ident::synthetic(stringify!($name).to_string()),
            $crate::item::Member::Tuple(vec![])
        )
    }};
    ($name:ident($($arg:expr), * $(,)?)) => {{
        use $crate::components::*;
        (
            $crate::expr::Ident::synthetic(stringify!($name).to_string()),
            $crate::item::Member::Tuple(vec![$($arg,)*])
        )
    }};
}

/// Creates a dedicated set of operators stemming from various TokKinds.
#[macro_export]
macro_rules! op {
    (#[doc = $doc:expr]$name:ident { $($tok:ident => $op:ident), * $(,)? }) => {
        #[doc = $doc]
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub enum $name {
            $(
                $op,
            )*
        }

        impl TryFrom<$crate::lex::TokKind<'_>> for $name {
            type Error = ();

            fn try_from(value: $crate::lex::TokKind<'_>) -> Result<Self, Self::Error> {
                match value {
                    $($crate::lex::TokKind::$tok => Ok($name::$op),)*
                    _ => Err(()),
                }
            }
        }

        impl<'s> From<$name> for $crate::lex::TokKind<'s> {
            fn from(value: $name) -> Self {
                match value {
                    $($name::$op => $crate::lex::TokKind::$tok,)*
                }
            }
        }

        #[mutants::skip]
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $($name::$op => f.pad(&$crate::lex::TokKind::from(*self).to_string()),)*
                }
            }
        }
    };
}
