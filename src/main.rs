use std::env;
use std::fs;
use std::process;

use kria::lexer::Lexer;
use kria::parser::Parser;
use kria::compiler::Compiler;
use kria::vm::VM;

fn print_usage(program: &str) {
    eprintln!("Kria programming language");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  {}              Start interactive REPL", program);
    eprintln!("  {} <file.krx>   Run a Kria source file", program);
    eprintln!("  {} -h, --help   Show this help", program);
}

fn run_file(filename: &str) -> Result<(), String> {
    let source = fs::read_to_string(filename)
        .map_err(|e| format!("Error reading file '{}': {}", filename, e))?;

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();

    let mut parser = Parser::new(tokens);
    let statements = parser.parse()?;

    let compiler = Compiler::new();
    let bytecode = compiler.compile(&statements)?;

    let mut vm = VM::new();
    vm.execute(&bytecode)?;
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        if let Err(e) = kria::repl::run() {
            eprintln!("REPL error: {}", e);
            process::exit(1);
        }
        return;
    }

    match args[1].as_str() {
        "-h" | "--help" => {
            print_usage(&args[0]);
        }
        path => {
            if let Err(e) = run_file(path) {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
    }
}
