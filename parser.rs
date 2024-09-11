use regex::Regex;
use std::sync::{ Arc, RwLock };
use crate::utils::node::{ NodeClass, Node };
use crate::utils::error::{ ErrorClass, Err };


fn current_tok<'a> (cfg: ParserConfig<'a>) -> Node { cfg.data.read().unwrap().current_token.clone() }


// used for fields that need to have a static length (if condition, loop iterator/index)
fn field_len<'a> (cfg: ParserConfig<'a>) -> usize { cfg.data.read().unwrap().stack.last().unwrap().branches.len() }


fn str_to_re(pattern: &str) -> Regex { Regex::new(&format!(r"^(?<token>{})(?<whitespace>[\s]*)", pattern)).unwrap() }


#[derive(Clone)]
pub struct ParserData {
	pub stack: Vec<Node>,
	current_token: Node,
	pub index: usize,
}

#[derive(Clone)]
pub struct ParserConfig<'a> {
	pub data: Arc<RwLock<ParserData>>, // dynamic objects placed in shared reference
	pub source: &'a str,
	token_patterns: [(regex::Regex, &'a str, Option<u8>); 18]
}

impl <'a> ParserConfig<'a> {
	pub fn create (source: &'a str) -> ParserConfig<'a> {
		let config = ParserConfig {
			data: Arc::new(RwLock::new(
				ParserData {
					stack: Vec::new(),
					current_token: Node::new(NodeClass::Misc(String::new()), [0, 0], Vec::new()),
					index: 0
			})),
			source,
			token_patterns: [
				(str_to_re(r"\'[^\']*\'"), "STRING", None),
				(str_to_re(r#"\"[^\"]*\""#), "STRING", None),
				(str_to_re(r"(True|False)"), "BOOLEAN", None),
				(str_to_re(r"(Void)"), "VOID", None),
				(str_to_re(r"[a-zA-Z][a-zA-Z0-9_]*"), "SYMBOL", None),
				(str_to_re(r"\[[!@]\]"), "SYMBOL", None),
				(str_to_re(r"[0-9]+\.?[0-9]*"), "NUMBER", None),
				
				(str_to_re(r"<\-[=<>\+\-\*/%\^]?"), "OPER", Some(5)),
				(str_to_re(r"[<>!]="), "OPER", Some(5)),
				(str_to_re(r"[=<>]"), "OPER", Some(5)),
				
				(str_to_re(r"<!?^>"), "OPER", Some(3)),
				(str_to_re(r"<!?\+>"), "OPER", Some(4)),
				(str_to_re(r"<!?:>"), "OPER", Some(5)),
				
				(str_to_re(r"[\+\-]"), "OPER", Some(4)),
				(str_to_re(r"[\*/%]"), "OPER", Some(3)),
				(str_to_re(r"\^"), "OPER", Some(2)),
				(str_to_re(r":"), "OPER", Some(1)),
				
				(str_to_re(r"[^\s]"), "", None)
		]};
		let _ = get_token(config.clone());
		config
	}
}


fn get_token<'a> (cfg: ParserConfig<'a>) -> Result<(), Err<'a>> {
	let update_index = cfg.data.read().unwrap().current_token.span[1];
    cfg.data.write().unwrap().index = update_index;
    if cfg.source.len() < update_index { return Err(Err::parse_err(ErrorClass::EndOfFile)); }
    
    let mut value: (&str, usize) = ("", 0);
    let mut meta: (&str, Option<u8>) = ("", None);
    
    for pattern in cfg.token_patterns.iter() {
		match pattern.0.captures(&cfg.source[cfg.data.read().unwrap().index..]) {
			Some(tok) => { 
				value = (tok.name("token").unwrap().clone().as_str(), tok.name("whitespace").unwrap().len());
				meta = (pattern.1, pattern.2);
				break;
			},
			None => ()
	}}
    
    match value.0 {
		"" => { return Err(Err::parse_err(ErrorClass::UnknownToken)); },
		_ => ()
	};
    
    let signature: NodeClass = match meta.0 {
        "STRING" => {
            let mut chars = value.0.chars();
            chars.next();
            chars.next_back();
            NodeClass::String(chars.as_str().to_string())
        },
        "OPER" => {
        	match meta.1 {
        		Some(p) => NodeClass::Oper(value.0.to_string(), p),
        		None => { panic!("something went wrong"); }
        }},
        "NUMBER" => NodeClass::Number(value.0.parse::<f32>().unwrap()),
        "BOOLEAN" => NodeClass::Boolean(value.0.to_lowercase().parse::<bool>().unwrap()),
        "SYMBOL" => NodeClass::Symbol(value.0.to_string()),
        "VOID" => NodeClass::Void,
        "" => NodeClass::Misc(value.0.to_string()),
        _ => NodeClass::Misc(String::new())
    };
    {
		let mut config = cfg.data.write().unwrap();
		let index = config.index;
		config.current_token = Node::new(signature, [index, index+value.0.len()+value.1], Vec::new());
    }
    Ok(())
}


pub fn parse<'a> (cfg: ParserConfig<'a>) -> Result<(), Err<'a>> {
	let init_idx = cfg.data.read().unwrap().index;
	match current_tok(cfg.clone()).id() {
		("if", "Symbol") => if_stmnt(cfg.clone())?,
		("loop", "Symbol") => loop_stmnt(cfg.clone())?,
		("obj", "Symbol") => object_stmnt(cfg.clone())?,
		("(", "MiscCharacter") => {
			field(cfg.clone(), "(", ")", None)?;
			reduce(cfg.clone(), NodeClass::Paren, 1, init_idx);
		},
		("[", "MiscCharacter") => {
			field(cfg.clone(), "[", "]", Some(","))?;
			reduce(cfg.clone(), NodeClass::List, 1, init_idx);
		},
		("{", "MiscCharacter") => field(cfg.clone(), "{", "}", None)?,
		("-", "Operator") => {
			get_token(cfg.clone())?;
			let num_tok = current_tok(cfg.clone());
			let num = if let NodeClass::Number(v) = *num_tok.signature { v }else { 0.0 };
			cfg.data.write().unwrap().stack.push(Node::new(NodeClass::Number((0.0-num) as f32), num_tok.span, Vec::new()));
			get_token(cfg.clone())?;
		},
		(other, "MiscCharacter") | (other, "Operator") => { return Err(Err::parse_err(ErrorClass::UnknownSyntax(other.to_string()))); }
		_ => {
			let token = current_tok(cfg.clone());
			cfg.data.write().unwrap().stack.push(token);
			get_token(cfg.clone())?;
		}
	}
	Ok(())
}


fn oper_expr<'a> (cfg: ParserConfig<'a>) -> Result<(), Err<'a>> {
	let init_idx = cfg.data.read().unwrap().index;
	let mut oper_stack: Vec<Node> = Vec::new();
	loop {
		parse(cfg.clone())?;
		// parse calls and combinators
		if ("(", "MiscCharacter") == current_tok(cfg.clone()).id() {
			field(cfg.clone(), "(", ")", Some(","))?;
			if ("(", "MiscCharacter") == current_tok(cfg.clone()).id() {
				field(cfg.clone(), "(", ")", Some(","))?;
				reduce(cfg.clone(), NodeClass::Combinator, 3, init_idx);
			}
			else { reduce(cfg.clone(), NodeClass::Call, 2, init_idx); }
		}
		{
			let mut config_writer = cfg.data.write().unwrap();
			let p_curr = if let NodeClass::Oper(_, priority) = *config_writer.current_token.signature { priority }else { break; };
			
			if oper_stack.len() == 0 { oper_stack.push(config_writer.current_token.clone()); }
			else if let NodeClass::Oper(operator, priority) = *oper_stack.last().unwrap().signature.clone() { 
				if p_curr < priority { oper_stack.push(config_writer.current_token.clone()); }
				if p_curr >= priority {
					let r_op = config_writer.stack.pop().unwrap();
					let l_op = config_writer.stack.pop().unwrap();
					config_writer.stack.push( Node::new(NodeClass::OperExpr(operator), [l_op.span[0], r_op.span[1]], vec![l_op, r_op]) );
					oper_stack.pop();
					oper_stack.push(config_writer.current_token.clone());
		}}}
		get_token(cfg.clone())?;
	}
	{
		let mut config_writer = cfg.data.write().unwrap();
		for operator in oper_stack.iter().rev() {
		    let r_op = config_writer.stack.pop().unwrap();
		    let l_op = config_writer.stack.pop().unwrap();
		    let oper = if let NodeClass::Oper(ref op, _) = *operator.signature { op }else { panic!("something went wrong"); };
		    config_writer.stack.push( Node::new(NodeClass::OperExpr(oper.to_string()), [l_op.span[0], r_op.span[1]], vec![l_op, r_op]) );
	}}
	Ok(())
}


fn reduce<'a> (cfg: ParserConfig<'a>, class: NodeClass, length: usize, init_idx: usize) {
    {
    	let mut config = cfg.data.write().unwrap();

		let offset = config.stack.len() - length;
		let tree: Vec<Node> = config.stack[offset..config.stack.len()].to_vec();
		config.stack.truncate(offset);
		let final_idx = config.index;
		config.stack.push(Node::new(class, [init_idx, final_idx], tree));
	}
}


fn field<'a> (cfg: ParserConfig<'a>, start: &'a str, end: &'a str, delim: Option<&'a str>) -> Result<(), Err<'a>> {
	if (start, "MiscCharacter") != current_tok(cfg.clone()).id() { return Ok(()); }
	let init_idx = cfg.data.read().unwrap().index;
	let mut length = 0;
	get_token(cfg.clone())?;

	while (end, "MiscCharacter") != current_tok(cfg.clone()).id() {
		oper_expr(cfg.clone())?;
		length += 1;
		
		match delim {
			None => (),
			Some(sep) => {
				if (sep, "MiscCharacter") == current_tok(cfg.clone()).id() { get_token(cfg.clone())?; }
				else if (end, "MiscCharacter") == current_tok(cfg.clone()).id() { break; } // don't expect delim after last element
				else { return Err(Err::parse_err(ErrorClass::MissingSeperator(sep.to_string(), cfg.data.read().unwrap().stack.last().unwrap().show()))); }
		}}
	}
	get_token(cfg.clone())?;
	reduce(cfg, NodeClass::Field, length, init_idx);
	Ok(())
}


fn if_stmnt<'a> (cfg: ParserConfig<'a>) -> Result<(), Err<'a>> {
	let init_idx = cfg.data.read().unwrap().index;
	let mut length = 0;
	
	while ("if", "Symbol") == current_tok(cfg.clone()).id() {
		get_token(cfg.clone())?;
		field(cfg.clone(), "(", ")", None)?;
		if field_len(cfg.clone()) != 1 { return Err(Err::parse_err(ErrorClass::ResolutionFailure("IF", String::from("one boolean condition required for each if statement")))); }
		
		field(cfg.clone(), "{", "}", None)?; // action if true
		if field_len(cfg.clone()) < 1 { return Err(Err::parse_err(ErrorClass::ResolutionFailure("IF", String::from("action if true block cannot be empty")))); }
        
        match current_tok(cfg.clone()).id() {
            (",", "MiscCharacter") => {
                get_token(cfg.clone())?;
                reduce(cfg.clone(), NodeClass::Field, 2, init_idx)
            },
            ("{", "MiscCharacter") => {
                field(cfg.clone(), "{", "}", None)?;  // action if false (opt)
                if field_len(cfg.clone()) < 1 { return Err(Err::parse_err(ErrorClass::ResolutionFailure("IF", String::from("action if false block cannot be empty")))); }
                reduce(cfg.clone(), NodeClass::Field, 3, init_idx)
            },
            _ => reduce(cfg.clone(), NodeClass::Field, 2, init_idx)
    	}
    	length += 1;
    }
    reduce(cfg, NodeClass::If, length, init_idx);
    Ok(())
}


fn loop_stmnt<'a> (cfg: ParserConfig<'a>) -> Result<(), Err<'a>> {
    let init_idx = cfg.data.read().unwrap().index;
    get_token(cfg.clone())?;
    
    match current_tok(cfg.clone()).id() {
        ("cond", "Symbol") => {
            get_token(cfg.clone())?;
            field(cfg.clone(), "(", ")", None)?; // condition
            if field_len(cfg.clone()) != 1 { return Err(Err::parse_err(ErrorClass::ResolutionFailure("LOOP", String::from("one boolean condition required")))); }
            
            field(cfg.clone(), "{", "}", None)?; // loop action
            if field_len(cfg.clone()) < 1 { return Err(Err::parse_err(ErrorClass::ResolutionFailure("LOOP", String::from("loop action cannot be empty")))); }
            reduce(cfg.clone(), NodeClass::Loop("cond".to_string()), 2, init_idx)
        },
        ("iter", "Symbol") => {
        	get_token(cfg.clone())?;
            let mut length = 0;
            while ("(", "MiscCharacter") == current_tok(cfg.clone()).id() {
                field(cfg.clone(), "(", ")", Some(","))?; // iterator/ index pairs
                if field_len(cfg.clone()) != 2 { return Err(Err::parse_err(ErrorClass::ResolutionFailure("LOOP", String::from("index/ iterator pair required")))); }
                length += 1;
            }
            field(cfg.clone(), "{", "}", None)?; // loop action
            if field_len(cfg.clone()) < 1 { return Err(Err::parse_err(ErrorClass::ResolutionFailure("LOOP", String::from("loop action cannot be empty")))); }
            length += 1;
            reduce(cfg.clone(), NodeClass::Loop("iter".to_string()), length, init_idx)
        },
        _ => {
        	let bad_loop_type = cfg.data.read().unwrap().stack.last().unwrap().show();
            return Err(Err::parse_err(ErrorClass::ResolutionFailure("LOOP", format!("invalid loop type: {}", bad_loop_type))));
	}}
	Ok(())
}


fn object_stmnt<'a> (cfg: ParserConfig<'a>) -> Result<(), Err<'a>> {
    let init_idx = cfg.data.read().unwrap().index;
    get_token(cfg.clone())?;
    field(cfg.clone(), "(", ")", Some(","))?; // args
    field(cfg.clone(), "{", "}", None)?; // contents
    if field_len(cfg.clone()) < 1 { return Err(Err::parse_err(ErrorClass::ResolutionFailure("OBJECT", String::from("object contents cannot be empty")))); }

    reduce(cfg, NodeClass::Object, 2, init_idx);
    Ok(())
}

