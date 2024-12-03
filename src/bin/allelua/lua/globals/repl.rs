use mlua::Lua;
use rustyline::{
    validate::{ValidationContext, ValidationResult},
    Completer, Helper, Highlighter, Hinter,
};
use tokio::select;
use tokio_util::sync::CancellationToken;

pub async fn repl(lua: &Lua) -> mlua::Result<()> {
    let mut ed = rustyline::Editor::with_history(
        rustyline::Config::default(),
        rustyline::history::MemHistory::default(),
    )
    .expect("Failed to create REPL editor");
    ed.set_helper(Some(ReplValidator::default()));

    let token = CancellationToken::new();
    let token_clone = token.clone();

    lua.globals().set(
        "close",
        lua.create_function(move |_, ()| {
            token_clone.cancel();
            Ok(())
        })?,
    )?;

    let mut interrupted = false;

    'repl: loop {
        match ed.readline("🙏 ") {
            Ok(line) => {
                let _ = ed.add_history_entry(line.clone());
                let line = line.trim();
                let line = line.strip_prefix("local ").unwrap_or(line);
                select! {
                    _ = token.cancelled() => {
                        break 'repl;
                    }
                    res = lua.load(line).eval_async::<mlua::Value>() => match res {
                        Ok(v) => {
                            println!("{}", v.to_string().unwrap_or_else(|err| err.to_string()))
                        }
                        Err(err) => eprintln!("{err}"),
                    }
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) => {
                if interrupted {
                    break;
                }
                println!("press ctrl+c again to exit");
                interrupted = true;
                continue;
            }
            Err(rustyline::error::ReadlineError::Eof) => break,
            Err(err) => panic!("{1}: {:?}", err, "Failed to read line"),
        }
        interrupted = false;

        if token.is_cancelled() {
            break;
        }
    }

    lua.globals().set("close", mlua::Value::Nil)?;
    Ok(())
}

#[derive(Default, Completer, Helper, Highlighter, Hinter)]
struct ReplValidator {}

impl rustyline::validate::Validator for ReplValidator {
    fn validate(&self, ctx: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        match full_moon::parse(ctx.input()) {
            Ok(_) => Ok(ValidationResult::Valid(None)),
            Err(errs) => {
                let mut result = ValidationResult::Valid(None);

                for err in errs {
                    match err {
                        full_moon::Error::AstError(err) => {
                            let msg = err.to_string();

                            if msg.contains("unexpected token")
                                && err.token().token_kind() == full_moon::tokenizer::TokenKind::Eof
                            {
                                result = ValidationResult::Incomplete;
                                break;
                            }

                            result = ValidationResult::Invalid(Some(format!("\n{msg}")));
                            break;
                        }
                        full_moon::Error::TokenizerError(err) => match err.error() {
                            full_moon::tokenizer::TokenizerErrorType::UnclosedComment
                            | full_moon::tokenizer::TokenizerErrorType::UnclosedString => {
                                if let ValidationResult::Valid(_) = result {
                                    result = ValidationResult::Incomplete;
                                }
                            }
                            err => {
                                result = ValidationResult::Invalid(Some(format!("\n{err}")));
                                break;
                            }
                        },
                    }
                }

                Ok(result)
            }
        }
    }

    fn validate_while_typing(&self) -> bool {
        false
    }
}
