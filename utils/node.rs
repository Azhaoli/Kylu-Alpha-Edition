use crate::utils::error::{ ErrorClass, Err };


// must be valid after the source that created them is out of scope
// so all fields must be owned types
#[derive(Debug, Clone, PartialEq)]
pub enum NodeClass {
    // primitive types
    String(String),
    Number(f32), // integer 
    Boolean(bool),
    Symbol(String),
    Oper(String, u8), // u8 operator precedeb
    Misc(String),
    Void,
    
    // compound types
    OperExpr(String),
    ObjectInst(String),
    Loop(String),
    Field,
    List,
    Paren,
    Object,
    Combinator,
    Call,
    If,
}


#[derive(Debug, Clone)]
pub struct Node {
    pub signature: Box<NodeClass>,
    pub span: [usize; 2],
    pub branches: Vec<Box<Node>>
}

impl PartialEq for Node {
	fn eq(&self, other: &Node) -> bool {
		if self.signature != other.signature { return false; }
		if self.branches != other.branches { return false; }
		return true;
	}
}

impl Node {
    pub fn new (sign: NodeClass, span: [usize; 2], contents: Vec<Node>) -> Node {
        Node { signature: Box::new(sign), span, branches: contents.into_iter().map(|node| Box::new(node)).collect() }
    }
    
    // used when creating a new node from an existing node's contents
    pub fn new_boxed (sign: NodeClass, span: [usize; 2], branches: Vec<Box<Node>>) -> Node { Node { signature: Box::new(sign), span, branches } }
    
    // shortcuts for creating common node types
    pub fn void () -> Node { Node::new(NodeClass::Void, [0, 0], Vec::new()) }
    
    pub fn symbol (name: String) -> Node { Node::new(NodeClass::Symbol(name), [0, 0], Vec::new()) }
    
    pub fn string (name: String) -> Node { Node::new(NodeClass::String(name), [0, 0], Vec::new()) }
    
    pub fn number (val: f32) -> Node { Node::new(NodeClass::Number(val), [0, 0], Vec::new()) }
    
    pub fn boolean (b: bool) -> Node { Node::new(NodeClass::Boolean(b), [0, 0], Vec::new()) }
    
    // unwrap node to inner value 
    pub fn into_number<'a> (&self) -> Result<f32, Err<'a>> {
    	match *self.signature {
    		NodeClass::Number(num) => Ok(num),
    		_ => Err(Err::new(ErrorClass::TypeMismatch("Number", self.get_type()), self.clone()))
    }}
    
    pub fn into_boolean<'a> (&self) -> Result<bool, Err<'a>> {
    	match *self.signature {
    		NodeClass::Boolean(b) => Ok(b),
    		_ => Err(Err::new(ErrorClass::TypeMismatch("Boolean", self.get_type()), self.clone()))
    }}
    
    pub fn into_string<'a> (&self) -> Result<String, Err<'a>> {
    	match *self.signature {
    		NodeClass::String(ref strg) => Ok(strg.to_string()),
    		NodeClass::Symbol(ref strg) => Ok(strg.to_string()),
    		_ => Err(Err::new(ErrorClass::TypeMismatch("String", self.get_type()), self.clone()))
    }}
    
    // commonly used node property checcks
    pub fn validate_type<'a> (&self, expected: &'a str) -> Result<Node, Err<'a>> {
		if self.get_type() == expected { Ok(self.clone()) }else { Err(Err::new(ErrorClass::TypeMismatch(expected, self.get_type()), self.clone())) }
	}
	
	pub fn validate_args_len<'a> (&self, len: usize) -> Result<Node, Err<'a>> {
		if self.branches.len() == len { Ok(self.clone()) }else { Err(Err::new(ErrorClass::ArgMismatch(len, self.branches.len()), self.clone())) }
	}
    
    // default representation for debugging
    pub fn show (&self) -> String {
        format!("({}{})",
            match *self.signature {
                NodeClass::String(ref val) => format!("str: {}", val),
                NodeClass::Number(ref val) => format!("num: {}", val),
                NodeClass::Boolean(ref val) => format!("bool: {}", val),
                NodeClass::Symbol(ref val) => format!("sym: {}", val),
                NodeClass::Oper(ref val, _) => format!("oper: {}", val),
                
                NodeClass::OperExpr(ref val) => format!("opex: {}", val),
                NodeClass::ObjectInst(ref val) => format!("inst: {}", val),
                NodeClass::Field => String::from("[!]"),
                NodeClass::List => String::from("[_]"),
                NodeClass::Paren => String::from("(_)"),
                NodeClass::Object => String::from("obj"),
                NodeClass::Combinator => String::from("com"),
                NodeClass::Call => String::from("call"),
                NodeClass::If => String::from("ifs"),
                NodeClass::Loop(ref t) => format!("loop: {}", t),
                NodeClass::Misc(ref val) => String::from(val),
                NodeClass::Void => String::from("void"),
            },
            if self.branches.len() != 0 { format!(" -->{}", self.branches.iter().map(|tok| tok.show()).collect::<String>()) }
            else { String::new() }
		)
	}
	
	// used for terminal readout (write() function)
	pub fn decode (&self) -> String {
		match *self.signature {
		    NodeClass::String(ref val) => String::from(val),
		    NodeClass::Number(ref val) => format!("{}", val),
		    NodeClass::Boolean(ref val) => format!("{}", val),
		    NodeClass::Symbol(ref val) => String::from(val),
		    NodeClass::Oper(ref val, _) => String::from(val),

		    NodeClass::Field => format!("{}", self.branches.iter().map(|elem| elem.decode()).reduce(|total, elem| total + ", " + &elem).unwrap()),
		    NodeClass::List => format!("[{}]", self.branches.iter().map(|elem| elem.decode()).reduce(|total, elem| total + ", " + &elem).unwrap()),
		    NodeClass::Paren => format!("({})", self.branches.iter().map(|elem| elem.decode()).reduce(|total, elem| total + ", " + &elem).unwrap()),
		    
		    _ => self.show()
	}}
	
	// user interface to identify nodes (type() function)
	pub fn get_type<'a> (&self) -> &'a str {
		match *self.signature {
			NodeClass::String(_) => "String",
            NodeClass::Number(_) => "Number",
            NodeClass::Boolean(_) => "Boolean",
            NodeClass::Symbol(_) => "Symbol",
            NodeClass::Oper(_, _) => "Operator",
            
            NodeClass::OperExpr(_) => "OperatorExpression",
            NodeClass::ObjectInst(_) => "ObjectInstance",
            NodeClass::Field => "Field",
            NodeClass::List => "List",
            NodeClass::Paren => self.branches[0].branches[0].get_type(), // get type of inner object
            NodeClass::Object => "Object",
            NodeClass::Combinator => "Combinator",
            NodeClass::Call => "Call",
            NodeClass::If => "IfExpression",
            NodeClass::Loop(_) => "LoopExpression",
            NodeClass::Misc(_) => "MiscCharacter",
            NodeClass::Void => "Void",
	}}
	
	// internal interface for identifying nodes
	// idk how im supposed to match NodeClass idiomatically
	pub fn id<'a> (&'a self) -> (&'a str, &'a str) {
		match *self.signature {
			NodeClass::String(ref a) => (a, "String"),
            NodeClass::Symbol(ref a) => (a, "Symbol"),
            NodeClass::Oper(ref a, _) => (a, "Operator"),
            
            NodeClass::OperExpr(ref a) => (a, "OperatorExpression"),
            NodeClass::ObjectInst(ref a) => (a, "ObjectInstance"),
            NodeClass::Loop(ref a) => (a, "LoopExpression"),
            NodeClass::Misc(ref a) => (a, "MiscCharacter"),
            NodeClass::Paren => ("_", "Parenthesis"), // prevent inner type passthrough
            _ => ("_", self.get_type())
	}}
}

