use crate::lexer::Token;
use crate::ast::{Statement, Expression, Literal, BinaryOperator, UnaryOperator};

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            position: 0,
        }
    }
    
    fn current_token(&self) -> &Token {
        self.tokens.get(self.position).unwrap_or(&Token::Eof)
    }
    
    fn advance(&mut self) {
        if self.position < self.tokens.len() {
            self.position += 1;
        }
    }
    
    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if std::mem::discriminant(self.current_token()) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected {:?}, found {:?}", expected, self.current_token()))
        }
    }
    
    fn expect_statement_end(&mut self) -> Result<(), String> {
        match self.current_token() {
            Token::Newline => {
                self.advance();
                Ok(())
            }
            Token::Eof | Token::RBrace => Ok(()),
            _ => Err(format!("Expected newline or end of file, found {:?}", self.current_token())),
        }
    }
    
    pub fn parse(&mut self) -> Result<Vec<Statement>, String> {
        let mut statements = Vec::new();
        
        while self.current_token() != &Token::Eof {
            while self.current_token() == &Token::Newline {
                self.advance();
            }
            if self.current_token() == &Token::Eof {
                break;
            }
            statements.push(self.parse_statement()?);
        }
        
        Ok(statements)
    }
    
    fn parse_statement(&mut self) -> Result<Statement, String> {
        match self.current_token() {
            Token::Set => self.parse_assignment(),
            Token::Print => self.parse_print(),
            Token::If => self.parse_if_statement(),
            Token::While => self.parse_while_statement(),
            Token::Fn => self.parse_function_def(),
            Token::Return => self.parse_return(),
            Token::Identifier(_) | Token::Number(_) | Token::String(_) | Token::True | Token::False | Token::Null | Token::LParen => {
                // Try to parse as expression statement (e.g., function call)
                let expr = self.parse_expression()?;
                self.expect_statement_end()?;
                Ok(Statement::Expression(expr))
            }
            _ => Err(format!("Unexpected token: {:?}", self.current_token())),
        }
    }
    
    fn parse_while_statement(&mut self) -> Result<Statement, String> {
        self.expect(Token::While)?;
        let condition = self.parse_expression()?;
        while self.current_token() == &Token::Newline {
            self.advance();
        }
        let body = self.parse_block()?;

        Ok(Statement::While { condition, body })
    }

    fn parse_if_statement(&mut self) -> Result<Statement, String> {
        self.expect(Token::If)?;
        let condition = self.parse_expression()?;
        while self.current_token() == &Token::Newline {
            self.advance();
        }
        let true_branch = self.parse_block()?;
        
        while self.current_token() == &Token::Newline {
            self.advance();
        }

        let mut branches = vec![(condition, true_branch)];
        let mut else_branch = None;

        while let Token::ElseIf = self.current_token() {
            self.advance();
            let condition = self.parse_expression()?;
            while self.current_token() == &Token::Newline {
                self.advance();
            }
            let branch = self.parse_block()?;
            while self.current_token() == &Token::Newline {
                self.advance();
            }
            branches.push((condition, branch));
        }

        if let Token::Else = self.current_token() {
            self.advance();
            while self.current_token() == &Token::Newline {
                self.advance();
            }
            else_branch = Some(self.parse_block()?);
        }

        Ok(Statement::If {
            branches,
            else_branch,
        })
    }

    fn parse_block(&mut self) -> Result<Vec<Statement>, String> {
        self.expect(Token::LBrace)?;

        let mut statements = Vec::new();
        while self.current_token() != &Token::RBrace {
            if self.current_token() == &Token::Newline {
                self.advance();
                continue;
            }
            statements.push(self.parse_statement()?);
            if self.current_token() == &Token::Newline {
                self.advance();
            }
        }

        self.expect(Token::RBrace)?;
        Ok(statements)
    }

    fn parse_assignment(&mut self) -> Result<Statement, String> {
        self.expect(Token::Set)?;
        
        let name = match self.current_token() {
            Token::Identifier(n) => {
                let n_clone = n.clone();
                self.advance();
                n_clone
            }
            _ => return Err("Expected identifier after 'set'".to_string()),
        };
        
        while self.current_token() == &Token::Newline {
            self.advance();
        }
        
        self.expect(Token::Equal)?;
        let value = self.parse_expression()?;
        self.expect_statement_end()?;
        
        Ok(Statement::Assignment { name, value })
    }
    
    fn parse_print(&mut self) -> Result<Statement, String> {
        self.expect(Token::Print)?;
        self.expect(Token::LParen)?;
        let expr = self.parse_expression()?;
        self.expect(Token::RParen)?;
        self.expect_statement_end()?;
        
        Ok(Statement::Print(expr))
    }

    fn parse_function_def(&mut self) -> Result<Statement, String> {
        self.expect(Token::Fn)?;
        
        let name = match self.current_token() {
            Token::Identifier(n) => {
                let n_clone = n.clone();
                self.advance();
                n_clone
            }
            _ => return Err("Expected function name after 'fn'".to_string()),
        };
        
        self.expect(Token::LParen)?;
        let params = self.parse_parameter_list()?;
        self.expect(Token::RParen)?;
        
        while self.current_token() == &Token::Newline {
            self.advance();
        }
        
        let body = self.parse_block()?;
        self.expect_statement_end()?;
        
        Ok(Statement::FunctionDef { name, params, body })
    }

    fn parse_return(&mut self) -> Result<Statement, String> {
        self.expect(Token::Return)?;
        
        let expr = match self.current_token() {
            Token::Newline | Token::RBrace | Token::Eof => None,
            _ => Some(self.parse_expression()?),
        };
        
        self.expect_statement_end()?;
        Ok(Statement::Return(expr))
    }

    fn parse_parameter_list(&mut self) -> Result<Vec<String>, String> {
        let mut params = Vec::new();
        
        while self.current_token() != &Token::RParen {
            if let Token::Identifier(name) = self.current_token() {
                params.push(name.clone());
                self.advance();
                
                if self.current_token() == &Token::RParen {
                    break;
                }
                // Allow comma-separated parameters (optional)
                if matches!(self.current_token(), Token::Comma) {
                    self.advance();
                }
            } else {
                return Err("Expected parameter name".to_string());
            }
        }
        
        Ok(params)
    }
    
    fn parse_expression(&mut self) -> Result<Expression, String> {
        self.parse_or()
    }
    
    fn parse_or(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_and()?;

        while let Token::Or = self.current_token() {
            self.advance();
            let right = self.parse_and()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op: BinaryOperator::Or,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_equality()?;

        while let Token::And = self.current_token() {
            self.advance();
            let right = self.parse_equality()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op: BinaryOperator::And,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_comparison()?;

        while let Token::EqualEqual = self.current_token() {
            self.advance();
            let right = self.parse_comparison()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op: BinaryOperator::Equals,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_additive()?;

        loop {
            let op = match self.current_token() {
                Token::Greater => Some(BinaryOperator::GreaterThan),
                Token::Less => Some(BinaryOperator::LessThan),
                Token::GreaterEqual => Some(BinaryOperator::GreaterOrEqual),
                Token::LessEqual => Some(BinaryOperator::LessOrEqual),
                Token::NotEqual => Some(BinaryOperator::NotEquals),
                _ => None,
            };

            if let Some(op) = op {
                self.advance();
                let right = self.parse_additive()?;
                left = Expression::BinaryOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                };
                continue;
            }
            break;
        }

        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_multiplicative()?;
        
        loop {
            match self.current_token() {
                Token::Plus => {
                    self.advance();
                    let right = self.parse_multiplicative()?;
                    left = Expression::BinaryOp {
                        left: Box::new(left),
                        op: BinaryOperator::Add,
                        right: Box::new(right),
                    };
                }
                Token::Minus => {
                    self.advance();
                    let right = self.parse_multiplicative()?;
                    left = Expression::BinaryOp {
                        left: Box::new(left),
                        op: BinaryOperator::Subtract,
                        right: Box::new(right),
                    };
                }
                _ => break,
            }
        }
        
        Ok(left)
    }
    
    fn parse_multiplicative(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_unary()?;
        
        loop {
            match self.current_token() {
                Token::Star => {
                    self.advance();
                    let right = self.parse_primary()?;
                    left = Expression::BinaryOp {
                        left: Box::new(left),
                        op: BinaryOperator::Multiply,
                        right: Box::new(right),
                    };
                }
                Token::Slash => {
                    self.advance();
                    let right = self.parse_primary()?;
                    left = Expression::BinaryOp {
                        left: Box::new(left),
                        op: BinaryOperator::Divide,
                        right: Box::new(right),
                    };
                }
                _ => break,
            }
        }
        
        Ok(left)
    }
    
    fn parse_unary(&mut self) -> Result<Expression, String> {
        match self.current_token() {
            Token::Not => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expression::UnaryOp {
                    op: UnaryOperator::Not,
                    expr: Box::new(expr),
                })
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Expression, String> {
        match self.current_token() {
            Token::Number(n) => {
                let n_val = *n;
                self.advance();
                Ok(Expression::Literal(Literal::Number(n_val)))
            }
            Token::String(s) => {
                let s_val = s.clone();
                self.advance();
                Ok(Expression::Literal(Literal::String(s_val)))
            }
            Token::True => {
                self.advance();
                Ok(Expression::Literal(Literal::Boolean(true)))
            }
            Token::False => {
                self.advance();
                Ok(Expression::Literal(Literal::Boolean(false)))
            }
            Token::Null => {
                self.advance();
                Ok(Expression::Literal(Literal::Null))
            }
            Token::Identifier(name) => {
                let name_val = name.clone();
                self.advance();
                
                // Check for function call
                if self.current_token() == &Token::LParen {
                    self.advance();
                    let args = self.parse_argument_list()?;
                    self.expect(Token::RParen)?;
                    Ok(Expression::FunctionCall {
                        name: name_val,
                        args,
                    })
                } else {
                    Ok(Expression::Identifier(name_val))
                }
            }
            Token::Fn => {
                self.advance();
                self.expect(Token::LParen)?;
                let params = self.parse_parameter_list()?;
                self.expect(Token::RParen)?;
                
                while self.current_token() == &Token::Newline {
                    self.advance();
                }
                
                let body = self.parse_block()?;
                Ok(Expression::FunctionExpr { params, body })
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            _ => Err(format!("Unexpected token in expression: {:?}", self.current_token())),
        }
    }

    fn parse_argument_list(&mut self) -> Result<Vec<Expression>, String> {
        let mut args = Vec::new();
        
        while self.current_token() != &Token::RParen {
            args.push(self.parse_expression()?);
            
            if self.current_token() == &Token::RParen {
                break;
            }
            
            if matches!(self.current_token(), Token::Comma) {
                self.advance();
            }
        }
        
        Ok(args)
    }
}