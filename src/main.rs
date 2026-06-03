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
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        let id = it.id.get_binding_identifier().unwrap().name.to_string();

        if self.id_type_vars.contains_key(&id) {
            panic!("A variable is being declared with a non-unique identifier!");
        }

        let left_operand = Type::TypeVar(id.clone());

        if let Some(_) = it.init {
            let right_operand = Type::TypeVar(it.node_id().index().to_string());
            self.non_id_type_vars
                .insert(it.node_id().index().to_string(), right_operand.clone());

            self.constraints.push((left_operand.clone(), right_operand));
        }
        self.id_type_vars.insert(id, left_operand);

        walk::walk_variable_declarator(self, it);
    }
    fn visit_formal_parameter(&mut self, it: &FormalParameter<'a>) {
        let id = it.pattern.get_identifier_name().unwrap().into_string();
        self.id_type_vars.insert(id.to_string(), Type::TypeVar(id));

        walk::walk_formal_parameter(self, it);
    }
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let Some(binding_id) = &it.id {
            let id = binding_id.name.into_string();
            self.id_type_vars.insert(id.to_string(), Type::TypeVar(id));
        }

        walk::walk_function(self, it, flags);
    }
    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        let node_id = it.node_id().index().to_string();
        let left_id = it.left.node_id().index().to_string();
        let left_operator = Type::TypeVar(left_id.clone());
        let right_id = it.right.node_id().index().to_string();
        let right_operator = Type::TypeVar(right_id.clone());

        self.non_id_type_vars.insert(node_id, Type::Int);
        self.non_id_type_vars.insert(left_id, left_operator.clone());
        self.non_id_type_vars
            .insert(right_id, right_operator.clone());

        if it.operator == Equality {
            // do stuff
        } else {
            self.constraints
                .push((left_operator.clone(), right_operator.clone()));
            self.constraints.push((left_operator, Type::Int));
            self.constraints.push((right_operator, Type::Int));
        }
    }
}

#[derive(Debug, Clone)]
enum Type {
    Int,
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
    println!("{:?}", visitor.id_type_vars);
    println!("{:?}", visitor.non_id_type_vars);
    println!("{:?}", visitor.constraints);
}

fn main() {
    let source = read_program("example.js").unwrap();

    let allocator = Allocator::default();
    let program = gen_ast(&allocator, &source);
    gen_constraints(program);
}
