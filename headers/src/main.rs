use ariadne::{sources, Color, Label, Report, ReportKind};
use chumsky::{input::ValueInput, prelude::*};
use std::{env, fmt, fs};

pub type Span = SimpleSpan;
pub type Spanned<T> = (T, Span);

#[derive(Clone, Debug, PartialEq)]
enum Token<'src> {
    // Keywords
    Struct,
    Class,
    Public,
    Private,
    Protected,
    Virtual,
    Template,
    Typename,

    // Basic types
    Int,
    Float,
    Double,
    Char,
    Bool,
    Void,

    // Identifiers and literals
    Ident(&'src str),

    // Operators and punctuation
    Colon,      // :
    Semicolon,  // ;
    Comma,      // ,
    Star,       // *
    Ampersand,  // &
    LeftParen,  // (
    RightParen, // )
    LeftBrace,  // {
    RightBrace, // }
    LeftAngle,  // <
    RightAngle, // >

    // Whitespace and comments (filtered out)
    Whitespace,
    Comment,
}

impl fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Token::Struct => write!(f, "struct"),
            Token::Class => write!(f, "class"),
            Token::Public => write!(f, "public"),
            Token::Private => write!(f, "private"),
            Token::Protected => write!(f, "protected"),
            Token::Virtual => write!(f, "virtual"),
            Token::Template => write!(f, "template"),
            Token::Typename => write!(f, "typename"),
            Token::Int => write!(f, "int"),
            Token::Float => write!(f, "float"),
            Token::Double => write!(f, "double"),
            Token::Char => write!(f, "char"),
            Token::Bool => write!(f, "bool"),
            Token::Void => write!(f, "void"),
            Token::Ident(s) => write!(f, "{s}"),
            Token::Colon => write!(f, ":"),
            Token::Semicolon => write!(f, ";"),
            Token::Comma => write!(f, ","),
            Token::Star => write!(f, "*"),
            Token::Ampersand => write!(f, "&"),
            Token::LeftParen => write!(f, "("),
            Token::RightParen => write!(f, ")"),
            Token::LeftBrace => write!(f, "{{"),
            Token::RightBrace => write!(f, "}}"),
            Token::LeftAngle => write!(f, "<"),
            Token::RightAngle => write!(f, ">"),
            Token::Whitespace => write!(f, " "),
            Token::Comment => write!(f, "//"),
        }
    }
}

fn lexer<'src>(
) -> impl Parser<'src, &'src str, Vec<Spanned<Token<'src>>>, extra::Err<Rich<'src, char, Span>>> {
    // Single character tokens
    let single_char = choice((
        just(':').to(Token::Colon),
        just(';').to(Token::Semicolon),
        just(',').to(Token::Comma),
        just('*').to(Token::Star),
        just('&').to(Token::Ampersand),
        just('(').to(Token::LeftParen),
        just(')').to(Token::RightParen),
        just('{').to(Token::LeftBrace),
        just('}').to(Token::RightBrace),
        just('<').to(Token::LeftAngle),
        just('>').to(Token::RightAngle),
    ));

    // Identifiers and keywords
    let ident = text::ascii::ident().map(|ident: &str| match ident {
        "struct" => Token::Struct,
        "class" => Token::Class,
        "public" => Token::Public,
        "private" => Token::Private,
        "protected" => Token::Protected,
        "virtual" => Token::Virtual,
        "template" => Token::Template,
        "typename" => Token::Typename,
        "int" => Token::Int,
        "float" => Token::Float,
        "double" => Token::Double,
        "char" => Token::Char,
        "bool" => Token::Bool,
        "void" => Token::Void,
        _ => Token::Ident(ident),
    });

    // Comments
    let line_comment = just("//")
        .then(any().and_is(just('\n').not()).repeated())
        .to(Token::Comment);

    let block_comment = just("/*")
        .then(any().and_is(just("*/").not()).repeated())
        .then(just("*/"))
        .to(Token::Comment);

    let comment = line_comment.or(block_comment);

    // Whitespace
    let whitespace = one_of(" \t\n\r")
        .repeated()
        .at_least(1)
        .to(Token::Whitespace);

    let token = single_char.or(ident).or(comment).or(whitespace);

    token
        .map_with(|tok, e| (tok, e.span()))
        .repeated()
        .collect()
        .map(|tokens: Vec<_>| {
            // Filter out whitespace and comments
            tokens
                .into_iter()
                .filter(|(tok, _)| !matches!(tok, Token::Whitespace | Token::Comment))
                .collect()
        })
}

#[derive(Clone, Debug, PartialEq)]
enum AccessSpecifier {
    Public,
    Private,
    Protected,
}

#[derive(Clone, Debug, PartialEq)]
enum DataType<'src> {
    Int,
    Float,
    Double,
    Char,
    Bool,
    Void,
    Custom(&'src str),
    Pointer(Box<DataType<'src>>),
    Reference(Box<DataType<'src>>),
    Template(&'src str, Vec<DataType<'src>>),
}

impl fmt::Display for DataType<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DataType::Int => write!(f, "int"),
            DataType::Float => write!(f, "float"),
            DataType::Double => write!(f, "double"),
            DataType::Char => write!(f, "char"),
            DataType::Bool => write!(f, "bool"),
            DataType::Void => write!(f, "void"),
            DataType::Custom(name) => write!(f, "{name}"),
            DataType::Pointer(inner) => write!(f, "{inner}*"),
            DataType::Reference(inner) => write!(f, "{inner}&"),
            DataType::Template(name, args) => {
                write!(f, "{name}<")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{arg}")?;
                }
                write!(f, ">")
            }
        }
    }
}

#[derive(Clone, Debug)]
struct Parameter<'src> {
    data_type: DataType<'src>,
    name: &'src str,
}

#[derive(Clone, Debug)]
struct Function<'src> {
    is_virtual: bool,
    return_type: DataType<'src>,
    name: &'src str,
    parameters: Vec<Parameter<'src>>,
}

#[derive(Clone, Debug)]
struct DataMember<'src> {
    data_type: DataType<'src>,
    name: &'src str,
}

#[derive(Clone, Debug)]
enum Member<'src> {
    Data(DataMember<'src>),
    Function(Function<'src>),
}

#[derive(Clone, Debug)]
struct AccessSection<'src> {
    access: AccessSpecifier,
    members: Vec<Member<'src>>,
}

#[derive(Clone, Debug)]
struct Inheritance<'src> {
    access: AccessSpecifier,
    base_class: &'src str,
}

#[derive(Clone, Debug)]
enum Declaration<'src> {
    Struct {
        template_params: Option<Vec<&'src str>>,
        name: &'src str,
        inheritance: Vec<Inheritance<'src>>,
        members: Vec<AccessSection<'src>>,
    },
    Class {
        template_params: Option<Vec<&'src str>>,
        name: &'src str,
        inheritance: Vec<Inheritance<'src>>,
        members: Vec<AccessSection<'src>>,
    },
}

fn parser<'src, I>(
) -> impl Parser<'src, I, Vec<Declaration<'src>>, extra::Err<Rich<'src, Token<'src>, Span>>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    let ident = select! { Token::Ident(name) => name };

    // Template parameter list
    let template_params = just(Token::Template)
        .ignore_then(just(Token::LeftAngle))
        .ignore_then(
            just(Token::Typename)
                .ignore_then(ident)
                .separated_by(just(Token::Comma))
                .collect::<Vec<_>>(),
        )
        .then_ignore(just(Token::RightAngle));

    // Data types
    let base_type = choice((
        just(Token::Int).to(DataType::Int),
        just(Token::Float).to(DataType::Float),
        just(Token::Double).to(DataType::Double),
        just(Token::Char).to(DataType::Char),
        just(Token::Bool).to(DataType::Bool),
        just(Token::Void).to(DataType::Void),
        ident.map(DataType::Custom),
    ));

    let data_type = recursive(|data_type| {
        // Template types
        let template_type = ident
            .then(
                just(Token::LeftAngle)
                    .ignore_then(
                        data_type
                            .clone()
                            .separated_by(just(Token::Comma))
                            .collect::<Vec<_>>(),
                    )
                    .then_ignore(just(Token::RightAngle)),
            )
            .map(|(name, args)| DataType::Template(name, args));

        let basic_type = template_type.or(base_type);

        // Pointer and reference types
        basic_type
            .then(
                choice((
                    just(Token::Star).repeated().at_least(1).to("*"),
                    just(Token::Ampersand).to("&"),
                ))
                .or_not(),
            )
            .map(|(base, modifier)| match modifier {
                Some("*") => DataType::Pointer(Box::new(base)),
                Some("&") => DataType::Reference(Box::new(base)),
                _ => base,
            })
    });

    // Access specifiers
    let access_specifier = choice((
        just(Token::Public).to(AccessSpecifier::Public),
        just(Token::Private).to(AccessSpecifier::Private),
        just(Token::Protected).to(AccessSpecifier::Protected),
    ));

    // Function parameters
    let parameter = data_type
        .clone()
        .then(ident)
        .map(|(data_type, name)| Parameter { data_type, name });

    let parameter_list = parameter
        .separated_by(just(Token::Comma))
        .collect::<Vec<_>>()
        .delimited_by(just(Token::LeftParen), just(Token::RightParen));

    // Functions
    let function = just(Token::Virtual)
        .or_not()
        .then(data_type.clone())
        .then(ident)
        .then(parameter_list)
        .then_ignore(just(Token::Semicolon))
        .map(|(((is_virtual, return_type), name), parameters)| Function {
            is_virtual: is_virtual.is_some(),
            return_type,
            name,
            parameters,
        });

    // Data members
    let data_member = data_type
        .clone()
        .then(ident)
        .then_ignore(just(Token::Semicolon))
        .map(|(data_type, name)| DataMember { data_type, name });

    // Members
    let member = function
        .map(Member::Function)
        .or(data_member.map(Member::Data));

    // Access sections
    let access_section = access_specifier
        .clone()
        .then_ignore(just(Token::Colon))
        .then(member.repeated().collect::<Vec<_>>())
        .map(|(access, members)| AccessSection { access, members });

    // Inheritance
    let inheritance_item = access_specifier
        .clone()
        .then(ident)
        .map(|(access, base_class)| Inheritance { access, base_class });

    let inheritance_list = just(Token::Colon)
        .ignore_then(
            inheritance_item
                .separated_by(just(Token::Comma))
                .collect::<Vec<_>>(),
        )
        .or_not()
        .map(|opt| opt.unwrap_or_default());

    // Struct declaration
    let struct_decl = template_params
        .clone()
        .or_not()
        .then(just(Token::Struct))
        .then(ident)
        .then(inheritance_list.clone())
        .then(
            just(Token::LeftBrace)
                .ignore_then(access_section.clone().repeated().collect::<Vec<_>>())
                .then_ignore(just(Token::RightBrace))
                .then_ignore(just(Token::Semicolon)),
        )
        .map(
            |((((template_params, _), name), inheritance), members)| Declaration::Struct {
                template_params,
                name,
                inheritance,
                members,
            },
        );

    // Class declaration
    let class_decl = template_params
        .clone()
        .or_not()
        .then(just(Token::Class))
        .then(ident)
        .then(inheritance_list.clone())
        .then(
            just(Token::LeftBrace)
                .ignore_then(access_section.clone().repeated().collect::<Vec<_>>())
                .then_ignore(just(Token::RightBrace))
                .then_ignore(just(Token::Semicolon)),
        )
        .map(
            |((((template_params, _), name), inheritance), members)| Declaration::Class {
                template_params,
                name,
                inheritance,
                members,
            },
        );

    let declaration = struct_decl.or(class_decl);

    declaration.repeated().collect().then_ignore(end())
}

fn print_declarations(declarations: &[Declaration]) {
    for decl in declarations {
        match decl {
            Declaration::Struct {
                template_params,
                name,
                inheritance,
                members,
            } => {
                if let Some(params) = template_params {
                    print!("template<");
                    for (i, param) in params.iter().enumerate() {
                        if i > 0 {
                            print!(", ");
                        }
                        print!("typename {param}");
                    }
                    println!(">");
                }

                print!("struct {name}");

                if !inheritance.is_empty() {
                    print!(" : ");
                    for (i, inh) in inheritance.iter().enumerate() {
                        if i > 0 {
                            print!(", ");
                        }
                        print!("{:?} {}", inh.access, inh.base_class);
                    }
                }

                println!(" {{");

                for section in members {
                    println!("  {:?}:", section.access);
                    for member in &section.members {
                        match member {
                            Member::Data(data) => {
                                println!("    {} {};", data.data_type, data.name);
                            }
                            Member::Function(func) => {
                                print!("    ");
                                if func.is_virtual {
                                    print!("virtual ");
                                }
                                print!("{} {}(", func.return_type, func.name);
                                for (i, param) in func.parameters.iter().enumerate() {
                                    if i > 0 {
                                        print!(", ");
                                    }
                                    print!("{} {}", param.data_type, param.name);
                                }
                                println!(");");
                            }
                        }
                    }
                }

                println!("}};");
                println!();
            }
            Declaration::Class {
                template_params,
                name,
                inheritance,
                members,
            } => {
                if let Some(params) = template_params {
                    print!("template<");
                    for (i, param) in params.iter().enumerate() {
                        if i > 0 {
                            print!(", ");
                        }
                        print!("typename {param}");
                    }
                    println!(">");
                }

                print!("class {name}");

                if !inheritance.is_empty() {
                    print!(" : ");
                    for (i, inh) in inheritance.iter().enumerate() {
                        if i > 0 {
                            print!(", ");
                        }
                        print!("{:?} {}", inh.access, inh.base_class);
                    }
                }

                println!(" {{");

                for section in members {
                    println!("  {:?}:", section.access);
                    for member in &section.members {
                        match member {
                            Member::Data(data) => {
                                println!("    {} {};", data.data_type, data.name);
                            }
                            Member::Function(func) => {
                                print!("    ");
                                if func.is_virtual {
                                    print!("virtual ");
                                }
                                print!("{} {}(", func.return_type, func.name);
                                for (i, param) in func.parameters.iter().enumerate() {
                                    if i > 0 {
                                        print!(", ");
                                    }
                                    print!("{} {}", param.data_type, param.name);
                                }
                                println!(");");
                            }
                        }
                    }
                }

                println!("}};");
                println!();
            }
        }
    }
}

fn main() {
    let filename = env::args().nth(1).expect("Expected file argument");
    let src = fs::read_to_string(&filename).expect("Failed to read file");

    let (tokens, errs) = lexer().parse(src.as_str()).into_output_errors();
    let errors = errs.into_iter().map(|e| e.map_token(|c| c.to_string()));

    let parse_errs = if let Some(tokens) = &tokens {
        let (ast, parse_errs) = parser()
            .parse(
                tokens
                    .as_slice()
                    .map((src.len()..src.len()).into(), |(t, s)| (t, s)),
            )
            .into_output_errors();

        if let Some(declarations) = ast {
            println!("Parsed C++ declarations:");
            println!("========================");
            print_declarations(&declarations);
            dbg!(declarations);
        }

        parse_errs
    } else {
        vec![]
    };

    let errors = errors.chain(
        parse_errs
            .into_iter()
            .map(|e| e.map_token(|tok| tok.to_string())),
    );

    // Print errors
    for e in errors {
        let e = e.map_token(|c| c.to_string());
        Report::build(ReportKind::Error, (filename.clone(), e.span().into_range()))
            .with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
            .with_message(e.to_string())
            .with_label(
                Label::new((filename.clone(), e.span().into_range()))
                    .with_message(e.reason().to_string())
                    .with_color(Color::Red),
            )
            .with_labels(e.contexts().map(|(label, span)| {
                Label::new((filename.clone(), span.into_range()))
                    .with_message(format!("while parsing this {label}"))
                    .with_color(Color::Yellow)
            }))
            .finish()
            .print(sources([(filename.clone(), src.clone())]))
            .unwrap()
    }
}
