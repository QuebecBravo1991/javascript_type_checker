use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_syntax::scope::ScopeFlags;
use std::collections::HashMap;
use std::fs;
use std::io;

struct UniqueId {
    current_id: i32,
}

impl UniqueId {
    fn new() -> Self {
        UniqueId { current_id: -1 }
    }
    fn next(&mut self) -> String {
        self.current_id += 1;
        return self.current_id.to_string();
    }
}

struct Visitor {
    id_type_vars: HashMap<String, Type>,
    non_id_type_vars: HashMap<String, Type>,
    constraints: Vec<(Type, Type)>,
    id_genor: UniqueId,
}

impl<'a> Visit<'a> for Visitor {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        let id = it.id.get_binding_identifier().unwrap().name.to_string();

        if self.id_type_vars.contains_key(&id) {
            panic!("A variable is being declared with a non-unique identifier!");
        }

        if let Some(_) = it.init {
            let left_operand = Type::TypeVar(id.to_string());
            let right_operand = Type::TypeVar(self.id_genor.next());
            self.constraints.push((left_operand, right_operand));
        }
        self.id_type_vars
            .insert(id.to_string(), Type::TypeVar(id.to_string()));

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
    // all non identifier expressions get a type variable
    // fn visit_expression
}

#[derive(Debug)]
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

fn gen_ast(source: String) {
    let allocator = Allocator::default();
    let source_type = SourceType::default();
    let ret = Parser::new(&allocator, &source, source_type).parse();
    let program = ret.program;

    let mut visitor = Visitor {
        id_type_vars: HashMap::new(),
        non_id_type_vars: HashMap::new(),
        constraints: Vec::new(),
        id_genor: UniqueId::new(),
    };
    visitor.visit_program(&program);
    println!("{:?}", visitor.id_type_vars);
    println!("{:?}", visitor.constraints);
}

fn main() {
    let source = read_program("example.js").unwrap();
    gen_ast(source);
}
