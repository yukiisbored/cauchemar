extern crate pest;
#[macro_use]
extern crate pest_derive;

use std::{collections::HashMap, fs, fmt, path::PathBuf};

use pest::Parser;

#[derive(Parser)]
#[grammar = "cauchemar.pest"]
struct CauchemarParser;

#[derive(Debug)]
enum CauchemarAST<'a> {
    Number(i32),
    Bool(bool),
    String(&'a str),
    Identifier(&'a str),
    If(Vec<CauchemarAST<'a>>, Vec<CauchemarAST<'a>>),
    While(Vec<CauchemarAST<'a>>),
    Add,
    Sub,
    Mul,
    Div,
}

impl fmt::Display for CauchemarAST<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CauchemarAST::Number(n) => write!(f, "{}", n),
            CauchemarAST::Bool(true) => write!(f, "TRUE"),
            CauchemarAST::Bool(false) => write!(f, "FALSE"),
            CauchemarAST::String(s) => write!(f, "\"{}\"", s),
            CauchemarAST::Identifier(s) => write!(f, "{}", s),
            CauchemarAST::If(then, otherwise) => {
                write!(f, "IF ")?;
                for c in then {
                    write!(f, "{} ", c)?;
                }
                write!(f, "ELSE ")?;
                for o in otherwise {
                    write!(f, "{} ", o)?;
                }
                write!(f, "THEN")
            },
            CauchemarAST::While(body) => {
                write!(f, "DO")?;
                for b in body {
                    write!(f, "{} ", b)?;
                }
                write!(f, "WHILE")
            },
            CauchemarAST::Add => write!(f, "+"),
            CauchemarAST::Sub => write!(f, "-"),
            CauchemarAST::Mul => write!(f, "*"),
            CauchemarAST::Div => write!(f, "/"),
        }
    }
}

#[derive(Debug)]
struct CauchemarProgram<'a> {
    routines: HashMap<&'a str, Vec<CauchemarAST<'a>>>,
}

fn parse_cauchemar_file(file: &str) -> Result<CauchemarProgram, pest::error::Error<Rule>> {
    let program = CauchemarParser::parse(Rule::program, file)?.next().unwrap();

    let mut routines = HashMap::new();

    use pest::iterators::Pair;

    fn parse_command(pair: Pair<Rule>) -> CauchemarAST {
        match pair.as_rule() {
            Rule::number => CauchemarAST::Number(pair.as_str().parse().unwrap()),
            Rule::string => CauchemarAST::String(pair.as_str().trim_matches('"')),
            Rule::identifier => CauchemarAST::Identifier(pair.as_str()),
            Rule::true_ => CauchemarAST::Bool(true),
            Rule::false_ => CauchemarAST::Bool(false),
            Rule::add => CauchemarAST::Add,
            Rule::sub => CauchemarAST::Sub,
            Rule::mul => CauchemarAST::Mul,
            Rule::div => CauchemarAST::Div,
            Rule::if_block => {
                let mut pairs = pair.into_inner();
                let then = pairs.next().unwrap().into_inner().map(parse_command).collect();
                let otherwise = match pairs.next() {
                    Some(o) => o.into_inner().map(parse_command).collect(),
                    None => vec![],
                };
                CauchemarAST::If(then, otherwise)
            }
            Rule::while_block => {
                let body = pair.into_inner().map(parse_command).collect();
                CauchemarAST::While(body)
            }
            _ => unreachable!(),
        }
    }

    for routine in program.into_inner() {
        match routine.as_rule() {
            Rule::routine => {
                let mut routine_rules = routine.into_inner();
                let routine_name = routine_rules.next().unwrap().as_str();
                let mut routine_ast = Vec::new();

                for command in routine_rules {
                    routine_ast.push(parse_command(command));
                }

                routines.insert(routine_name, routine_ast);
            }
            Rule::EOI => (),
            _ => unreachable!(),
        }
    }

    Ok(CauchemarProgram { routines })
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CauchemarVMValue<'a> {
    Number(i32),
    Bool(bool),
    String(&'a str),
}

impl fmt::Display for CauchemarVMValue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CauchemarVMValue::Number(n) => write!(f, "{}", n),
            CauchemarVMValue::Bool(true) => write!(f, "TRUE"),
            CauchemarVMValue::Bool(false) => write!(f, "FALSE"),
            CauchemarVMValue::String(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug)]
enum CauchemarVMInstruction<'a> {
    Push(CauchemarVMValue<'a>),
    Call(&'a str),
    Jump(usize),
    JumpIfFalse(usize),
    Add,
    Sub,
    Mul,
    Div,
    Return,
    Nop,
}

impl fmt::Display for CauchemarVMInstruction<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CauchemarVMInstruction::Push(v) => write!(f, "PUSH {}", v),
            CauchemarVMInstruction::Call(r) => write!(f, "CALL {}", r),
            CauchemarVMInstruction::Jump(i) => write!(f, "JUMP {}", i),
            CauchemarVMInstruction::JumpIfFalse(i) => write!(f, "JUMP_IF_FALSE {}", i),
            CauchemarVMInstruction::Add => write!(f, "ADD"),
            CauchemarVMInstruction::Sub => write!(f, "SUB"),
            CauchemarVMInstruction::Mul => write!(f, "MUL"),
            CauchemarVMInstruction::Div => write!(f, "DIV"),
            CauchemarVMInstruction::Return => write!(f, "RETURN"),
            CauchemarVMInstruction::Nop => write!(f, "NOP"),
        }
    }
}

enum CauchemarVMRoutine<'a> {
    Native(fn(&mut CauchemarVM<'a>)),
    User(Vec<CauchemarVMInstruction<'a>>),
}

impl fmt::Debug for CauchemarVMRoutine<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CauchemarVMRoutine::Native(_) => write!(f, "Native"),
            CauchemarVMRoutine::User(instructions) => {
                write!(f, "User({:?})", instructions)
            }
        }
    }
}

#[derive(Debug)]
struct CauchemarVM<'a> {
    ip: Vec<(&'a str, usize)>,
    stack: Vec<CauchemarVMValue<'a>>,
    routines: HashMap<&'a str, CauchemarVMRoutine<'a>>,
}

fn compile_cauchemar_program(program: CauchemarProgram) -> CauchemarVM {
    let mut routines = HashMap::new();

    fn compile_routine<'a>(instructions: &mut Vec<CauchemarVMInstruction<'a>>, routine: Vec<CauchemarAST<'a>>) {
        for command in routine {
            match command {
                CauchemarAST::Number(n) => instructions.push(CauchemarVMInstruction::Push(
                    CauchemarVMValue::Number(n),
                )),
                CauchemarAST::Bool(b) => instructions.push(CauchemarVMInstruction::Push(
                    CauchemarVMValue::Bool(b),
                )),
                CauchemarAST::String(s) => instructions.push(CauchemarVMInstruction::Push(
                    CauchemarVMValue::String(s),
                )),
                CauchemarAST::Identifier(s) => instructions.push(CauchemarVMInstruction::Call(s)),
                CauchemarAST::If(then, otherwise) => {
                    instructions.push(CauchemarVMInstruction::JumpIfFalse(0));
                    let false_jump_index = instructions.len() - 1;

                    compile_routine(instructions, then);
                    instructions.push(CauchemarVMInstruction::Jump(0));
                    let end_jump_index = instructions.len() - 1;

                    let false_jump = end_jump_index + 1;
                    compile_routine(instructions, otherwise);

                    instructions.push(CauchemarVMInstruction::Nop);
                    let end_jump = instructions.len() - 1;

                    instructions[false_jump_index] = CauchemarVMInstruction::JumpIfFalse(false_jump);
                    instructions[end_jump_index] = CauchemarVMInstruction::Jump(end_jump);
                }
                CauchemarAST::While(body) => {
                    let start_index = instructions.len();
                    compile_routine(instructions, body);
                    instructions.push(CauchemarVMInstruction::JumpIfFalse(0));
                    let false_jump_index = instructions.len() - 1;
                    instructions.push(CauchemarVMInstruction::Jump(start_index));

                    instructions.push(CauchemarVMInstruction::Nop);
                    let false_jump = instructions.len() - 1;

                    instructions[false_jump_index] = CauchemarVMInstruction::JumpIfFalse(false_jump);
                }
                CauchemarAST::Add => instructions.push(CauchemarVMInstruction::Add),
                CauchemarAST::Sub => instructions.push(CauchemarVMInstruction::Sub),
                CauchemarAST::Mul => instructions.push(CauchemarVMInstruction::Mul),
                CauchemarAST::Div => instructions.push(CauchemarVMInstruction::Div),
            }
        }
    }

    for (name, routine) in program.routines {
        let mut compiled_routine = Vec::new();
        compile_routine(&mut compiled_routine, routine);
        compiled_routine.push(CauchemarVMInstruction::Return);
        routines.insert(name, CauchemarVMRoutine::User(compiled_routine));
    }

    routines.insert("PRINT", CauchemarVMRoutine::Native(|vm| {
        match vm.stack.pop() {
            Some(v) => println!("{}", v),
            None => panic!("Stack underflow"),
        }
    }));

    routines.insert("DROP", CauchemarVMRoutine::Native(|vm| {
        vm.stack.pop();
    }));

    routines.insert("DUP", CauchemarVMRoutine::Native(|vm| {
        let value = match vm.stack.pop() {
            Some(value) => value,
            None => panic!("Stack underflow"),
        };
        vm.stack.push(value);
        vm.stack.push(value);
    }));

    routines.insert("SWAP", CauchemarVMRoutine::Native(|vm| {
        let a = match vm.stack.pop() {
            Some(value) => value,
            None => panic!("Stack underflow"),
        };
        let b = match vm.stack.pop() {
            Some(value) => value,
            None => panic!("Stack underflow"),
        };
        vm.stack.push(a);
        vm.stack.push(b);
    }));

    routines.insert("ROT", CauchemarVMRoutine::Native(|vm| {
        let a = match vm.stack.pop() {
            Some(value) => value,
            None => panic!("Stack underflow"),
        };
        let b = match vm.stack.pop() {
            Some(value) => value,
            None => panic!("Stack underflow"),
        };
        let c = match vm.stack.pop() {
            Some(value) => value,
            None => panic!("Stack underflow"),
        };
        vm.stack.push(b);
        vm.stack.push(a);
        vm.stack.push(c);
    }));

    routines.insert("OVER", CauchemarVMRoutine::Native(|vm| {
        let a = match vm.stack.pop() {
            Some(value) => value,
            None => panic!("Stack underflow"),
        };
        let b = match vm.stack.pop() {
            Some(value) => value,
            None => panic!("Stack underflow"),
        };
        vm.stack.push(b);
        vm.stack.push(a);
        vm.stack.push(b);
    }));

    routines.insert("EQUALS", CauchemarVMRoutine::Native(|vm| {
        let a = match vm.stack.pop() {
            Some(value) => value,
            None => panic!("Stack underflow"),
        };
        let b = match vm.stack.pop() {
            Some(value) => value,
            None => panic!("Stack underflow"),
        };

        vm.stack.push(CauchemarVMValue::Bool(a == b));
    }));

    routines.insert("NOT", CauchemarVMRoutine::Native(|vm| {
        let value = match vm.stack.pop() {
            Some(CauchemarVMValue::Bool(b)) => CauchemarVMValue::Bool(!b),
            Some(_) => panic!("Invalid type"),
            None => panic!("Stack underflow"),
        };

        vm.stack.push(value);
    }));

    routines.insert("OR", CauchemarVMRoutine::Native(|vm| {
        let a = match vm.stack.pop() {
            Some(CauchemarVMValue::Bool(b)) => b,
            Some(_) => panic!("Invalid type"),
            None => panic!("Stack underflow"),
        };
        let b = match vm.stack.pop() {
            Some(CauchemarVMValue::Bool(b)) => b,
            Some(_) => panic!("Invalid type"),
            None => panic!("Stack underflow"),
        };

        vm.stack.push(CauchemarVMValue::Bool(a || b));
    }));

    routines.insert("AND", CauchemarVMRoutine::Native(|vm| {
        let a = match vm.stack.pop() {
            Some(CauchemarVMValue::Bool(b)) => b,
            Some(_) => panic!("Invalid type"),
            None => panic!("Stack underflow"),
        };
        let b = match vm.stack.pop() {
            Some(CauchemarVMValue::Bool(b)) => b,
            Some(_) => panic!("Invalid type"),
            None => panic!("Stack underflow"),
        };

        vm.stack.push(CauchemarVMValue::Bool(a && b));
    }));

    fn number_comparison<F>(vm: &mut CauchemarVM, f: F)
    where
        F: Fn(i32, i32) -> bool,
    {
        let b = match vm.stack.pop() {
            Some(CauchemarVMValue::Number(n)) => n,
            Some(_) => panic!("Invalid type"),
            None => panic!("Stack underflow"),
        };

        let a = match vm.stack.pop() {
            Some(CauchemarVMValue::Number(n)) => n,
            Some(_) => panic!("Invalid type"),
            None => panic!("Stack underflow"),
        };

        vm.stack.push(CauchemarVMValue::Bool(f(a, b)));
    }

    routines.insert("GREATER-THAN", CauchemarVMRoutine::Native(|vm| number_comparison(vm, |a, b| a > b)));
    routines.insert("GREATER-EQUAL", CauchemarVMRoutine::Native(|vm| number_comparison(vm, |a, b| a >= b)));
    routines.insert("LESS-THAN", CauchemarVMRoutine::Native(|vm| number_comparison(vm, |a, b| a < b)));
    routines.insert("LESS-EQUAL", CauchemarVMRoutine::Native(|vm| number_comparison(vm, |a, b| a <= b)));

    routines.insert("ASSERT", CauchemarVMRoutine::Native(|vm| {
        let value = match vm.stack.pop() {
            Some(CauchemarVMValue::Bool(b)) => b,
            Some(_) => panic!("Invalid type"),
            None => panic!("Stack underflow"),
        };

        if !value {
            panic!("Assertion failed");
        }
    }));

    CauchemarVM {
        ip: vec![("PROGRAM", 0)],
        stack: Vec::new(),
        routines,
    }
}

fn binop<F>(vm: &mut CauchemarVM, f: F)
where
    F: Fn(i32, i32) -> i32,
{
    let b = match vm.stack.pop() {
        Some(CauchemarVMValue::Number(n)) => n,
        Some(_) => panic!("Invalid type"),
        None => panic!("Stack underflow"),
    };

    let a = match vm.stack.pop() {
        Some(CauchemarVMValue::Number(n)) => n,
        Some(_) => panic!("Invalid type"),
        None => panic!("Stack underflow"),
    };

    vm.stack.push(CauchemarVMValue::Number(f(a, b)));
}

fn run_vm(vm: &mut CauchemarVM) {
    loop {
        let (routine_name, ip) = vm.ip.pop().unwrap();

        let routine = match vm.routines.get(routine_name) {
            Some(routine) => routine,
            None => panic!("Unknown routine: {}", routine_name),
        };

        vm.ip.push((routine_name, ip + 1));

        match routine {
            CauchemarVMRoutine::Native(native) => {
                native(vm);
                vm.ip.pop();
            }
            CauchemarVMRoutine::User(instructions) => {
                let instruction = &instructions[ip];

                #[cfg(feature = "debug")]
                {
                    println!("[{:>5}] {}", ip, instruction);
                    println!("        STACK: {:?}", vm.stack);
                    println!("        ROUTINE: {:?}", routine_name);
                    println!("        FRAMES: {:?}", vm.ip);
                }

                match instruction {
                    CauchemarVMInstruction::Push(n) => vm.stack.push(*n),
                    CauchemarVMInstruction::Add => binop(vm, |a, b| a + b),
                    CauchemarVMInstruction::Sub => binop(vm, |a, b| a - b),
                    CauchemarVMInstruction::Mul => binop(vm, |a, b| a * b),
                    CauchemarVMInstruction::Div => binop(vm, |a, b| a / b),
                    CauchemarVMInstruction::Jump(pos) => {
                        vm.ip.pop();
                        vm.ip.push((routine_name, *pos));
                    }
                    CauchemarVMInstruction::JumpIfFalse(pos) => {
                        match vm.stack.pop() {
                            Some(CauchemarVMValue::Bool(false)) => {
                                vm.ip.pop();
                                vm.ip.push((routine_name, *pos));
                            }
                            Some(CauchemarVMValue::Bool(true)) => {}
                            Some(_) => panic!("Invalid type"),
                            None => panic!("Stack underflow"),
                        }
                    }
                    CauchemarVMInstruction::Call(routine_name) => vm.ip.push((routine_name, 0)),
                    CauchemarVMInstruction::Nop => {},
                    CauchemarVMInstruction::Return => {
                        vm.ip.pop();
                        if vm.ip.is_empty() {
                            break;
                        }
                    }
                }
            }
        }
    }

    if !vm.stack.is_empty() {
        for value in vm.stack.iter().rev() {
            println!("{}", value);
        }
    }
}

use clap::Parser as ClapParser;

#[derive(ClapParser)]
#[command(name = "cauchemar", about = "Cauchemar Interpreter", long_about = None)]
struct Cli {
    /// Cauchemar source file to run
    file: PathBuf,
}

fn main() {
    let cli = Cli::parse();
    let unparsed_file = fs::read_to_string(cli.file).expect("Unable to read file");
    let program = parse_cauchemar_file(&unparsed_file).expect("Unable to parse file");

    #[cfg(feature = "debug")]
    {
        println!("!!! PARSER OUTPUT !!!");
        for (routine_name, routine) in program.routines.iter() {
            print!("{}: ", routine_name);
            for ast in routine {
                print!("{} ", ast);
            }
            println!();
        }
    }

    if !program.routines.contains_key("PROGRAM") {
        panic!("Missing PROGRAM routine");
    }

    let mut vm = compile_cauchemar_program(program);

    #[cfg(feature = "debug")]
    {
        println!("!!! COMPILER OUTPUT !!!");
        for (routine_name, routine) in vm.routines.iter() {
            match routine {
                CauchemarVMRoutine::Native(_) => {},
                CauchemarVMRoutine::User(instructions) => {
                    println!("=== {} ===", routine_name);
                    for (i, instruction) in instructions.iter().enumerate() {
                        println!("[{:>5}] {}", i, instruction);
                    }
                }
            }
        }
        println!("!!! VM START !!!");
    }

    run_vm(&mut vm);
}
