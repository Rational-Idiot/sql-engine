use db::{
    lex::{Lex, Token},
    parser::Parser,
};

fn main() {
    let query = " SELECT u.id, u.name, u.score FROM users u WHERE u.score BETWEEN 10 AND 100 AND u.id IN (SELECT id FROM active_users) AND EXISTS ( SELECT * FROM logs l WHERE l.user_id = u.id AND l.action LIKE 'login%') OR u.name ILIKE 'A%' ORDER BY u.score DESC, u.name ASC LIMIT 10 OFFSET 5; ";

    let mut lexer = Lex::new();
    lexer.input = query.chars().collect();

    let tokens: Vec<Token> = lexer
        .map(|t| t.expect("Lexer error"))
        .take_while(|t| *t != Token::EOF)
        .chain(std::iter::once(Token::EOF))
        .collect();

    let mut parser = Parser::new(tokens);

    match parser.parse() {
        Ok(stmt) => {
            println!("── Parsed SQL ──\n");
            println!("{}", stmt);
        }
        Err(e) => {
            eprintln!("Parse Error: {}", e);
        }
    }
}

