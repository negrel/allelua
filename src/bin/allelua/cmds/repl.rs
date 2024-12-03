use std::path::PathBuf;

use clap::crate_version;
use rustyline::{
    history::MemHistory,
    validate::{ValidationResult, Validator},
    Completer, Config, Editor, Helper, Highlighter, Hinter,
};

use crate::lua::Runtime;

pub fn repl() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the tokio Runtime")
        .block_on(async {
            let fpath = PathBuf::from("/repl");
            let runtime = Runtime::new(&fpath, vec![]);

            let mut ed = Editor::with_history(Config::default(), MemHistory::default())
                .expect("Failed to create REPL editor");
            ed.set_helper(Some(InputValidator {}));

            println!("allelua {}", crate_version!());
            println!("exit using ctrl+d, ctrl+c or close()");

            // define close() as os.exit.
            runtime
                .exec::<()>(mlua::chunk! {
                    close = os.exit
                })
                .await
                .unwrap();

            let mut interrupted = false;
            loop {
                match ed.readline("🙏 ") {
                    Ok(line) => {
                        let _ = ed.add_history_entry(line.clone());
                        let line = line.trim();
                        let line = line.strip_prefix("local ").unwrap_or(line);
                        match runtime.exec::<mlua::Value>(line).await {
                            Ok(v) => {
                                println!("{}", v.to_string().unwrap_or_else(|err| err.to_string()))
                            }
                            Err(err) => eprintln!("{err}"),
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
            }
        })
}

#[derive(Completer, Helper, Highlighter, Hinter)]
struct InputValidator {}

impl Validator for InputValidator {
    fn validate(
        &self,
        ctx: &mut rustyline::validate::ValidationContext,
    ) -> rustyline::Result<ValidationResult> {
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
