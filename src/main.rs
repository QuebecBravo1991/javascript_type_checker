use oxc_allocator::Allocator;
use oxc_ast::ast::BinaryOperator::Equality;
use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_syntax::scope::ScopeFlags;
use std::collections::HashMap;
use std::fs;
use std::io;

struct Visitor {
    id_type_vars: HashMap<String, Type>,
    non_id_type_vars: HashMap<String, Type>,
    constraints: Vec<(Type, Type)>,
}

impl<'a> Visit<'a> for Visitor {
    // [[I]] = number
    fn visit_numeric_literal(&mut self, it: &NumericLiteral<'a>) {
        let node_id = it.node_id().index().to_string();
        self.non_id_type_vars
            .entry(node_id.clone())
            .or_insert(Type::Number);
        self.constraints
            .push((Type::TypeVar(node_id), Type::Number));
        walk::walk_numeric_literal(self, it);
    }
    // E1 op E2 and E1 == E2
    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        let node_id = it.node_id().index().to_string();
        let left_id = it.left.node_id().index().to_string();
        let left_operator = Type::TypeVar(left_id.clone());
        let right_id = it.right.node_id().index().to_string();
        let right_operator = Type::TypeVar(right_id.clone());

        self.non_id_type_vars
            .entry(node_id.clone())
            .or_insert(Type::Number);
        self.non_id_type_vars
            .entry(left_id.clone())
            .or_insert(left_operator.clone());
        self.non_id_type_vars
            .entry(right_id.clone())
            .or_insert(right_operator.clone());

        if it.operator != Equality {
            self.constraints.push((left_operator.clone(), Type::Number));
            self.constraints
                .push((right_operator.clone(), Type::Number));
        }
        self.constraints.push((left_operator, right_operator));
        self.constraints
            .push((Type::TypeVar(node_id), Type::Number));

        walk::walk_binary_expression(self, it);
    }
    // input
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if Some("parseInt") == it.callee_name() {
            let node_id = it.node_id().index().to_string();
            let expr_type = Type::TypeVar(node_id.clone());
            self.non_id_type_vars
                .entry(node_id.clone())
                .or_insert(expr_type.clone());
            self.constraints.push((expr_type, Type::Number));
        }
    }
    // X = E
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        let id = it.id.get_binding_identifier().unwrap().name.to_string();

        if self.id_type_vars.contains_key(&id) {
            panic!("Uh oh! A variable is being declared with a non-unique identifier!");
        }

        let left_operand = Type::TypeVar(id.clone());

        if let Some(_) = it.init {
            let right_operand_node_id = it.init.as_ref().unwrap().node_id().index().to_string();
            let right_operand = Type::TypeVar(right_operand_node_id.clone());
            self.non_id_type_vars
                .entry(right_operand_node_id)
                .or_insert(right_operand.clone());

            self.constraints.push((left_operand.clone(), right_operand));
        }
        self.id_type_vars.entry(id).or_insert(left_operand);

        walk::walk_variable_declarator(self, it);
    }
    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(id_ref) = &it.left {
            let left_type = Type::TypeVar(id_ref.name.to_string());
            let right_type;
            match &it.right {
                Expression::NumericLiteral(_) => right_type = Type::Number,
                Expression::Identifier(id_ref) => {
                    right_type = Type::TypeVar(id_ref.name.to_string())
                }
                Expression::CallExpression(ce) => {
                    right_type = Type::TypeVar(ce.node_id().index().to_string())
                }
                _ => panic!("Uh oh! Found a invalid expression in variable assignment"),
            }

            self.constraints.push((left_type, right_type));
        } else {
            panic!("Uh oh! An assignment is being made to something that is not a identifier.");
        }
        walk::walk_assignment_expression(self, it);
    }
    // TODO: output
    // TODO: if statements
    // fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
    //     walk::walk_if_statement(self, it);
    // }
    // TODO: if else statements
    // TODO: while statements
    // X(X1,. . . ,Xn ){ . . . return E; }
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let Some(binding_id) = &it.id {
            let id = binding_id.name.into_string();

            // Though this is JavaScript we are going to restrict the input program to TIP rules. The last statement it.body.as_ref().unwrap().statements.last().unwrap()in every function should be a return statement.
            if let Statement::ReturnStatement(x) =
                it.body.as_ref().unwrap().statements.last().unwrap()
            {
                let return_type;
                match x.argument.as_ref().unwrap() {
                    Expression::Identifier(id_ref) => {
                        let name = id_ref.name.to_string();
                        return_type = Type::TypeVar(name);
                    }
                    Expression::NumericLiteral(_) => return_type = Type::Number,
                    _ => panic!("Uh oh! The return type is not valid for this langauge subset."),
                }

                let mut param_type_vars = Vec::new();
                for param in &it.params.items {
                    if let BindingPattern::BindingIdentifier(binding_id) = &param.pattern {
                        let name = binding_id.name.to_string();
                        let param_type_var = Type::TypeVar(name.clone());
                        self.id_type_vars
                            .entry(name)
                            .or_insert(param_type_var.clone());
                        param_type_vars.push(param_type_var);
                    }
                }

                let id_type = Type::TypeVar(id.clone());
                self.id_type_vars.insert(id.clone(), id_type.clone());
                self.constraints.push((
                    id_type,
                    Type::Function(param_type_vars, Box::new(return_type)),
                ))
            } else {
                panic!("Uh oh! This function does not meet our TIP style restrictions.")
            }
        }

        walk::walk_function(self, it, flags);
    }
}

#[derive(Debug, Clone)]
enum Type {
    Number,
    Pointer(Box<Type>),
    Function(Vec<Type>, Box<Type>),
    MuExpression(String, Box<Type>),
    TypeVar(String),
}

fn read_program(path: &str) -> Result<String, io::Error> {
    let contents = fs::read_to_string(path)?;
    Ok(contents)
}

fn gen_ast<'a>(allocator: &'a Allocator, source: &'a str) -> Program<'a> {
    let source_type = SourceType::default();
    let program = Parser::new(allocator, source, source_type).parse().program;
    SemanticBuilder::new().build(&program);
    program
}

fn gen_constraints(program: Program) {
    let mut visitor = Visitor {
        id_type_vars: HashMap::new(),
        non_id_type_vars: HashMap::new(),
        constraints: Vec::new(),
    };

    visitor.visit_program(&program);

    println!("Identifier type variables");
    for var in visitor.id_type_vars {
        println!("{:?}", var)
    }
    println!();

    println!("Non identifier type variables");
    for var in visitor.non_id_type_vars {
        println!("{:?}", var);
    }
    println!();

    println!("Found constraints");
    for var in visitor.constraints {
        println!("{:?}", var);
    }
}

fn main() {
    let source = read_program("test_files/t1.js").unwrap();

    let allocator = Allocator::default();
    let program = gen_ast(&allocator, &source);
    gen_constraints(program);
}
