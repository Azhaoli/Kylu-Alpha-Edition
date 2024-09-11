use std::error::Error;
use std::{ fmt, process };
use crate::utils::node::{ NodeClass, Node };


// unlike nodes, errors are bound by the lifetime of the parser/ runtime that created them
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorClass<'a> {
	// parsing errors
	MissingSeperator(String, String), // sep, before
	UnmatchedBracket(String), // bracket
	UnknownToken,
	ResolutionFailure(&'a str, String), // failed element, reason
	EndOfFile,
	UnknownSyntax(String), // syntax identifier
	
	// runtime errors
	VoidReference(String), // name of invalid ref
	UndefinedOperation(String, &'a str, &'a str), // operation, left op type, right op type
	TypeMismatch(&'a str, &'a str), // expected type, found type
	IndexError(f32, String), // index, list
	ArgMismatch(usize, usize),
	Signal(&'a str), // signal identifier
	File(&'a str, String),
	Conversion(String, &'a str, &'a str), // converted node, start type, end type
	FatalError(String),
	CustomError(String) // used for errors that only apply in one particular context
}

#[derive(Debug, Clone)]
pub struct Err<'a> {
	pub class: Box<ErrorClass<'a>>,
	pub cause: Node,
}

impl <'a> Err<'a> {
	pub fn new (class: ErrorClass<'a>, cause: Node) -> Err<'a> { Err { class: Box::new(class), cause } }
	
	// enable user interaction with error values
	pub fn to_node(&self) -> Result<Node, Err<'a>> {
		match *self.class {
			ErrorClass::VoidReference(_) => { return Ok(Node::string("VoidReference".to_string())); },
			ErrorClass::UndefinedOperation(_, _, _) => { return Ok(Node::string("UndefinedOperation".to_string())); },
			ErrorClass::TypeMismatch(_, _) => { return Ok(Node::string("TypeMismatch".to_string())); },
			ErrorClass::IndexError(_, _) => { return Ok(Node::string("IndexError".to_string())); },
			ErrorClass::ArgMismatch(_, _) => { return Ok(Node::string("ArgMismatch".to_string())); },
			ErrorClass::Signal(_) => { return Ok(Node::string("Signal".to_string())); },
			ErrorClass::File(_, _) => { return Ok(Node::string("File".to_string())); },
			ErrorClass::Conversion(_, _, _) => { return Ok(Node::string("Conversion".to_string())); },
			ErrorClass::FatalError(_) => { return Ok(Node::string("FatalError".to_string())); },
			ErrorClass::CustomError(_) => { return Ok(Node::string("CustomError".to_string())); },
			_ => { return Err(Err::new(ErrorClass::CustomError("Parsing errors cannot be converted to nodes".to_string()), Node::void())); }
		}
	}

	
	// used for errors during parsing before context can be determined
	pub fn parse_err (class: ErrorClass<'a>) -> Err<'a> { Err { class: Box::new(class), cause: Node::void() } }
	
	pub fn throw (&self, source: String, trace: Vec<(NodeClass, [usize; 2])>, halt: bool) -> Node {
		
		eprintln!("----------------------------------- AN UNHANDLED EXCEPTION HAS OCCURRED! --------");
		match *self.class {
			// parsing errors
			ErrorClass::MissingSeperator(ref sep, ref before) => eprintln!("Expected seperator '{}' after element {}", sep, before),
			ErrorClass::UnmatchedBracket(ref bracket) => eprintln!("Bracket '{}' was never closed", bracket),
			ErrorClass::UnknownToken => eprintln!("Unrecognized token#"),
			ErrorClass::ResolutionFailure(elem, ref reason) => eprintln!("Failed to resolve element {}, {}", elem, reason),
			ErrorClass::EndOfFile => eprintln!("Scanner reached the end of the source file"),
			ErrorClass::UnknownSyntax(ref token) => println!("Unrecognized syntax identifier '{}'", token),
			
			// runtime errors
			ErrorClass::VoidReference(ref reference) => eprintln!("Reference '{}' has no associated value", reference),
			ErrorClass::UndefinedOperation(ref operation, l_op, r_op) => eprintln!("Operation '{}' is not defined for types {}, {}", operation, l_op, r_op), 
			ErrorClass::TypeMismatch(expected, found) => eprintln!("Expected type '{}', found type '{}'", expected, found),
			ErrorClass::IndexError(index, ref list) => eprintln!("Index {} is out of range for list {}",index, list),
			ErrorClass::ArgMismatch(expected, found) => eprintln!("Expected {} arguments, found {}", expected, found),
			ErrorClass::Signal(name) => eprintln!("Signal '{}' cannot be invoked outside it's associated block", name),
			ErrorClass::File(err, ref file) => eprintln!("An error occurred while processing the file '{}' {}", file, err),
			ErrorClass::Conversion(ref target, init, end) => eprintln!("{} '{}' cannot be converted to type {}", init, target, end),
			ErrorClass::FatalError(ref msg) => eprintln!("An unrecoverable error has occurred! {}", msg),
			ErrorClass::CustomError(ref msg) => eprintln!("{}", msg)
		}
		println!("---------------------------------------------------------------------------------");
		let mut source_slice = "";
		for call in trace.iter() {
			source_slice = &source[call.1[0]..call.1[1]];
			eprintln!("[-] ({}, {})----{}-> {}\n", call.1[0], call.1[1], Node::new(call.0.clone(), [0, 0], Vec::new()).show(), source_slice);
		}
		
		if halt { process::exit(1); }
		Node::void()
	}
}

impl fmt::Display for Err<'_> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "greetings mortals!!") }
}

impl Error for Err<'_> {}

