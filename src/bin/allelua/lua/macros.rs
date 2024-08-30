#[macro_export]
macro_rules! LuaTypeConstructors {
    ($vis:vis $type_name:ident $({ $($name:ident($($arg:ident: $arg_type:ty),* $(,)?) $body:block),* })? $(async { $($async_name:ident($($async_arg:ident: $async_arg_type:ty),* $(,)?) $async_body:block),* })?) => {
         $vis struct $type_name;

        impl UserData for $type_name {
            fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(_fields: &mut F) {}

            fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
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
