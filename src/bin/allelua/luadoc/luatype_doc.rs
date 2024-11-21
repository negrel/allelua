/// Parser define a luadoc type parser.
#[derive(Debug)]
pub struct Parser {}

/// Token define a luadoc type token.
enum Token {
    String,     // "foo"
    Word,       // string, number, nil
    LeftParen,  // (
    Comma,      // ,
    RightParen, // )
    Arrow,      // ->
    Pipe,       // |
    Amp,        // &
}
