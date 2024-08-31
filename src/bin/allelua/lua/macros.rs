#[macro_export]
macro_rules! LuaTypeConstructors {
    (
        $vis:vis $type_name:ident
        $({ $($name:ident($($arg:ident: $arg_type:ty),* $(,)?) $body:block),* })?
        $(async { $($async_name:ident($($async_arg:ident: $async_arg_type:ty),* $(,)?) $async_body:block),* })?
    ) => {
        #[derive(Debug, Clone)]
         $vis struct $type_name;

        impl UserData for $type_name {
            fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(_fields: &mut F) {}

            fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(#[allow(unused)] methods: &mut M) {
                $(
                    $(
                        methods.add_function(
                            stringify!($name),
                            |#[allow(unused)] lua, #[allow(unused_parens)] ($($arg),*): ($($arg_type),*)| $body
                        );
                    )*
                )*
                $(
                    $(
                        methods.add_async_function(
                            stringify!($async_name),
                            |#[allow(unused)] lua, #[allow(unused_parens)] ($($async_arg),*): ($($async_arg_type),*)| async move $async_body
                        );
                    )*
                )*
            }
        }
    };
}

#[macro_export]
macro_rules! LuaModule {
    (
        $type_name:ident,
        fields { $($field_name:ident = $field_value:expr)* },
        functions  { $($fn_name:ident($lua:ident $(, $fn_arg:ident: $fn_arg_type:ty)*) $body:block),* },
        async functions { $($async_fn_name:ident($async_lua:ident $(, $async_fn_arg:ident: $async_fn_arg_type:ty)*) $async_body:block),* }
    ) => {
        #[derive(Clone, mlua::FromLua)]
        pub struct $type_name;

        impl mlua::UserData for $type_name {
            fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(#[allow(unused)]fields: &mut F) {
                $(
                    fields.add_field(stringify!($field_name), $field_value);
                )*
            }

            fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(#[allow(unused)] methods: &mut M) {
                $(
                    methods.add_function(
                        stringify!($fn_name),
                        |$lua, #[allow(unused_parens)] ($($fn_arg),*): ($($fn_arg_type),*)| $body
                    );
                )*
                $(
                    methods.add_async_function(
                        stringify!($async_fn_name),
                        |$async_lua, #[allow(unused_parens)] ($($async_fn_arg),*): ($($async_fn_arg_type),*)| async move $async_body
                    );
                )*
            }
        }
    };
}
