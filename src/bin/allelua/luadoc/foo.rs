/// ModuleIdentifier define a unique lua module identifier.
/// An URL is used for user defined modules while stdlib uses short names such
/// as "sync" or "time".
#[derive(Debug)]
pub enum ModuleIdentifier {
    StdLib(String),
    Url(Url),
}

/// LuaModuleDoc holds documentation of an entire lua module.
#[derive(Debug)]
struct LuaModuleDoc {
    identifier: ModuleIdentifier,
    doc: String,
}

/// LuaFunctionDoc holds documentation of a lua function.
#[derive(Debug)]
struct LuaFunctionDoc {
    module: ModuleIdentifier,
    doc: String,
    name: String,
    parameters: Vec<String>,
}

/// LuaFunctionDoc holds documentation of a lua method.
#[derive(Debug)]
struct LuaMethodDoc {
    module: ModuleIdentifier,
}
