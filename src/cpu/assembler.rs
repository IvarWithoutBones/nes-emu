use crate::cpu::instructions::{instruction_identifier, instruction_name, INSTRUCTIONS};
use logos::Logos;

#[derive(Logos, Debug, PartialEq, Copy, Clone)]
enum Token {
    #[regex(r"([A-Za-z_])+")]
    Instr,

    #[regex(r"#")]
    Immediate,

    #[regex(r"\$[0-9a-fA-F]+")]
    Address,

    #[error]
    #[regex(r"[ \t\n\f]+", logos::skip)] // Skip whitespace
    Error,
}

#[derive(Debug)]
pub struct Assembler {
    // bytes: Vec<u8>,
    // position: usize,
}

impl Assembler {
    fn get_addr_literal(token: &str) -> u16 {
        let addr = &token[2..];
        u16::from_str_radix(addr, 16).unwrap()
    }

    pub fn new(assembly: &str) -> Self {
        let mut lexer = Token::lexer(assembly);
        let mut bytes = Vec::new();

        // dbg!(vec!(&lexer
        //     .clone()
        //     .map(|(token, span)| (token, span.start, span.end))
        //     .collect::<Vec<_>>(),));

        loop {
            let token = lexer.next();
            if token.is_none() {
                break;
            }

            let raw = &assembly[lexer.span()];

            match token {
                Some(Token::Instr) => {
                    let upper = raw.to_uppercase();
                    for instr in INSTRUCTIONS.iter() {
                        // Found instruction
                        if instruction_name(instr) == upper {
                            if let Some(next_token) = lexer.clone().peekable().peek() {
                                match next_token {
                                    // Immediate
                                    Token::Immediate => {
                                        lexer.next();
                                        bytes.push(
                                            instruction_identifier(
                                                instr,
                                                &crate::cpu::instructions::AdressingMode::Immediate,
                                            )
                                            .expect(
                                                "Invalid addressing mode implied for instruction",
                                            ),
                                        );

                                        let next_token = lexer.next();
                                        if next_token.is_none() {
                                            panic!("Expected literal after immediate addressing mode");
                                        }

                                        if let Token::Address = next_token.unwrap() {
                                            let addr = Self::get_addr_literal(raw);
                                            bytes.push((addr & 0xFF) as u8);
                                            bytes.push((addr >> 8) as u8);
                                        } else {
                                            panic!("Expected literal after immediate addressing mode");
                                        }
                                    }

                                    _ => {
                                        bytes.push(
                                            instruction_identifier(
                                                instr,
                                                &crate::cpu::instructions::AdressingMode::Implied,
                                            )
                                            .expect("Invalid instruction identifier"),
                                        );
                                    }
                                }
                            }

                            let (opcode, mode) = instr.2[0];
                            bytes.push(opcode); // TODO: dont assume first byte is opcode

                            println!("{}: {:02x}", raw, opcode);

                            if mode.has_arguments() {
                                let mut argument_count = mode.opcode_len() - 1;
                                while argument_count > 0 {
                                    println!("arg: {}", argument_count);
                                    let token = lexer.next();
                                    if token.is_none() {
                                        panic!("Unexpected end of input");
                                    }

                                    let raw_arg = &assembly[lexer.span()];
                                    println!("arg: {}", raw_arg);
                                    match token {
                                        Some(Token::Address) => {
                                            let real = Assembler::get_addr_literal(raw_arg);
                                            bytes.push((real & 0xFF) as u8);
                                            bytes.push(((real >> 8) & 0xFF) as u8);
                                        }
                                        _ => panic!("Unexpected token"),
                                    }

                                    argument_count = argument_count - 1;
                                }
                            }

                            break;
                        }
                    }
                }

                _ => {
                    panic!("Invalid token {:?}: {}", token, raw)
                }
            }
        }

        println!("\n");
        for byte in bytes.iter() {
            println!("{:#02x} ", byte);
        }

        // Self { bytes, position: 0 }
        Self {}
    }

    pub fn print(&self) {}
}
