use crate::{
    catalog::{Column, Table, catalog::Catalog},
    sql::{ast::*, lex::*, parser::*},
};

pub fn mock_catalog() -> Catalog {
    use std::collections::HashMap;

    let users = Table {
        name: "users".into(),
        name_lower: "users".into(),
        cols: vec![
            Column {
                id: 0,
                name: "id".into(),
                name_lower: "id".into(),
                data_type: DataType::Integer,
                nullable: false,
                primary_key: true,
                unique: true,
            },
            Column {
                id: 1,
                name: "age".into(),
                name_lower: "age".into(),
                data_type: DataType::Integer,
                nullable: false,
                primary_key: false,
                unique: false,
            },
            Column {
                id: 2,
                name: "score".into(),
                name_lower: "score".into(),
                data_type: DataType::Integer,
                nullable: false,
                primary_key: false,
                unique: false,
            },
            Column {
                id: 3,
                name: "name".into(),
                name_lower: "name".into(),
                data_type: DataType::String,
                nullable: false,
                primary_key: false,
                unique: false,
            },
            Column {
                id: 4,
                name: "active".into(),
                name_lower: "active".into(),
                data_type: DataType::Bool,
                nullable: false,
                primary_key: false,
                unique: false,
            },
            Column {
                id: 5,
                name: "deleted".into(),
                name_lower: "deleted".into(),
                data_type: DataType::Bool,
                nullable: false,
                primary_key: false,
                unique: false,
            },
            Column {
                id: 6,
                name: "banned".into(),
                name_lower: "banned".into(),
                data_type: DataType::Integer,
                nullable: false,
                primary_key: false,
                unique: false,
            },
        ],
    };

    let orders = Table {
        name: "orders".into(),
        name_lower: "orders".into(),
        cols: vec![
            Column {
                id: 0,
                name: "id".into(),
                name_lower: "id".into(),
                data_type: DataType::Integer,
                nullable: false,
                primary_key: true,
                unique: true,
            },
            Column {
                id: 1,
                name: "user_id".into(),
                name_lower: "user_id".into(),
                data_type: DataType::Integer,
                nullable: false,
                primary_key: false,
                unique: false,
            },
            Column {
                id: 2,
                name: "amount".into(),
                name_lower: "amount".into(),
                data_type: DataType::Float,
                nullable: false,
                primary_key: false,
                unique: false,
            },
        ],
    };

    let mut tables = HashMap::new();
    tables.insert("users".into(), users);
    tables.insert("orders".into(), orders);

    Catalog { tables }
}

pub fn parse(sql: &str) -> Stmt {
    let mut lexer = Lex::new();
    lexer.input = sql.chars().collect();

    let tokens: Vec<Token> = lexer
        .map(|t| t.unwrap())
        .take_while(|t| *t != Token::EOF)
        .chain(std::iter::once(Token::EOF))
        .collect();

    let mut parser = Parser::new(tokens);
    parser.parse().unwrap()
}
