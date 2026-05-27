use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use std::fs;
use std::io;

fn read_program(path: &str) -> Result<String, io::Error> {
    let contents = fs::read_to_string(path)?;
    Ok(contents)
}

fn gen_ast(source: String) {
    let allocator = Allocator::default();
    let source_type = SourceType::default();
    let ret = Parser::new(&allocator, &source, source_type).parse();
    let program = ret.program;

    println!("AST:");
    println!("{program:#?}");
}

fn main() {
    let source = read_program("example.js").unwrap();
    gen_ast(source);
}
