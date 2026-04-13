use db::sql::{
    lex::{Lex, Token},
    parser::Parser,
};

fn main() {
    let query = "SELECT -- Character Function to format the name INITCAP(first_name || ' ' || last_name) AS full_name, -- Numeric Function to round the income for readability ROUND(annual_income, -3) AS rounded_income, -- Data Mining Function to get the default probability PREDICTION_PROBABILITY(loan_default_model, 'YES' USING *) AS default_probability FROM Loan_applicant WHERE annual_income > 100000 ORDER BY default_probability DESC FETCH FIRST 5 ROWS ONLY;";

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
