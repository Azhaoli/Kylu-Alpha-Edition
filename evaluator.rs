use std::sync::{ Arc, RwLock };
use std::{ fs, process };
use crate::parser::{ ParserConfig, parse };
use crate::utils::node::{ NodeClass, Node };
use crate::utils::error::{ ErrorClass, Err };
use crate::utils::builtin_funcs::search_library;


#[derive(Clone)]
pub struct Env {
	pub trace: Vec<(NodeClass, [usize; 2])>,
    pub depth: usize,
    pub import: NameSpace,
    pub data: Vec<NameSpace>
}

impl Env {
    pub fn create () -> Env {
    	Env {
	        trace: Vec::new(),
	        depth: 0,
	        import: NameSpace::new(),
	        data: vec![NameSpace::new()]
		}
	}
}

#[derive(Debug, Clone)]
pub struct NameSpace { // double vec allows easy conversion to and from nodes
	keys: Arc<RwLock<Vec<Node>>>,
 	values: Arc<RwLock<Vec<Node>>>
}

impl <'a>NameSpace {
	pub fn new () -> NameSpace { NameSpace{ keys: Arc::new(RwLock::new(Vec::new())), values: Arc::new(RwLock::new(Vec::new())) } }
	
	fn from (keys: Node, values: Node) -> Result<NameSpace, Err<'a>> {
		let _ = keys.validate_args_len(values.branches.len() as usize)?;
		Ok(NameSpace {
			keys: Arc::new(RwLock::new(keys.branches.into_iter().map(|k| *k).collect())),
			values: Arc::new(RwLock::new(values.branches.into_iter().map(|k| *k).collect()))
		})
	}
	
	pub fn as_node (&self, label: Node) -> Node {
		let keys = Node::new(NodeClass::Field, [0, 0], self.keys.read().unwrap().clone());
		let values = Node::new(NodeClass::Field, [0, 0], self.values.read().unwrap().clone());
		let obj_name = label.into_string().unwrap_or_else(|_| "_".to_string());
		
		Node::new(NodeClass::ObjectInst(obj_name), [0, 0], vec![keys, values])
	}
	
	fn find_index (&self, symbol: Node) -> Result<usize, Err<'a>> {
		let lookup = symbol.clone();
		match self.keys.read().unwrap().iter().position(|key| *key.signature == *lookup.signature) {
			Some(result) => Ok(result),
			None => {
				let ref_name = if let (name, "Symbol") = symbol.id() { name.to_string() }else { "_".to_string() };
				Err(Err::new(ErrorClass::VoidReference(ref_name), symbol))
		}}
	}

	pub fn get (&self, key: Node) -> Result<Node, Err<'a>> {
		let index = self.find_index(key)?;
		Ok(self.values.read().unwrap()[index].clone())
	}
	
	pub fn set (&self, key: Node, value: Node) -> Result<Node, Err<'a>> {
		let index = self.find_index(key.clone());
		
		match index {
			Ok(idx) => { // update existing variable
				self.values.write().unwrap()[idx] = value.clone();
				Ok(value)
			},
			Err(e) => {
				if let ErrorClass::VoidReference(_) = *e.class { // create new variable
					self.keys.write().unwrap().push(key.clone());
					self.values.write().unwrap().push(value.clone());
					Ok(value)
				}
				else { Err(e) }
		}}
	}
	
	pub fn show(&self) -> String {
		let key_reader = self.keys.read().unwrap();
		let value_reader = self.values.read().unwrap();
		if key_reader.len() == 0 { return String::from("nothing to show"); }
		
		let mut string = String::new();
		for bind in 0..key_reader.len() {
			let var = format!("{:<20} <- {}\n", key_reader[bind].decode(), value_reader[bind].decode());
			string.push_str(&var);
		}
		string
	}
	
	pub fn show_modules(&self) {
		let key_reader = self.keys.read().unwrap();
		let value_reader = self.values.read().unwrap();
		if key_reader.len() == 0 {
			println!("nothing to show");
			return;
		}
		for file in 0..key_reader.len() {
			println!("-------------- BINDINGS IN MODULE {} --------", key_reader[file].decode());
			let module = value_reader[file].clone();
			let mod_keys = module.branches[0].branches.clone();
			let mod_values = module.branches[1].branches.clone();
			
			let mut string = String::new();
			for bind in 0..mod_keys.len() {
				let var = format!("{:<20} <- {}\n", mod_keys[bind].decode(), mod_values[bind].decode());
				string.push_str(&var);
			}
			println!("{}", string);
		}
	}
}


// load_file and eval_file required for the ext() function user interface
// i'd rather not import anything from crate::main so i'll just put them here
pub fn eval_file (env: Env, mut source_file: String, halt_on_err: bool) {
	source_file = format!("{{{}}}$", source_file); // ensure lookahead doesn't hit EOF
	let src = source_file.clone();
    let cfg = ParserConfig::create(&src);
    
    let result = match parse(cfg.clone()) {
    	Ok(()) => { cfg.data.write().unwrap().stack.pop().unwrap() },
    	Err(e) => {
    		let trace: Vec<(NodeClass, [usize; 2])> = cfg.data.read().unwrap().stack.iter().map(|elem| (*elem.signature.clone(), elem.span)).collect();
    		e.throw(source_file.clone(), trace, halt_on_err);
    		return;
    }};
    
	match evaluate(env.clone(), result) {
    	Ok(_) => (),
    	Err(e) => { e.throw(source_file, env.trace, halt_on_err); }
    };
}


pub fn load_file (path: String, halt_on_err: bool) -> String {
	fs::read_to_string(&path).unwrap_or_else( |err| {
		eprintln!("[-] an error occurred while opening the file {}: {}", &path, err);
		if halt_on_err { process::exit(1); }else { return String::new(); }
	})
}


fn handle_error<'a> (mut env: Env, val: Result<Node, Err<'a>>, expr: Node) -> Result<Node, Err<'a>> {
	let err = match val {
		Ok(node) => { return Ok(node); },
		Err(e) => { if e.to_node()?.id() != expr.branches[2].branches[0].id() { return Err(e); }else { e.to_node()? } }
	};
	
	env.data.push(env.data[env.depth].clone());
	env.depth += 1;  // prevent binding from leaking into outer scope

	let bind = expr.branches[1].validate_args_len(1)?.branches[0].validate_type("Symbol")?;
	env.data[env.depth].set(bind.clone(), err);
	let result = evaluate(env.clone(), *expr.branches[2].branches[1].clone());
	// println!("handled error: {:?}", result);
	
	env.depth -= 1;
	env.data.pop();
	return result;
}


fn add_extension<'a> (mut env: Env, tree: Node) -> Result<Node, Err<'a>> {
	let path = tree.branches[1].branches[0].into_string()?;
	let source = load_file(path.clone(), false);
	if source == String::new() { return Err(Err::new(ErrorClass::File("file not found", path), tree)); }
	
	let guest_env = Env::create();
	eval_file(guest_env.clone(), source, false);
	
	let file_name = path.split("/").collect::<Vec<&str>>().pop().unwrap(); // get last arg in path
	let mut label = file_name.split(".").nth(0).unwrap(); // remove extension
	
	env.import.set(Node::symbol(label.to_string()), guest_env.data[0].as_node(Node::symbol("<extension>".to_string())));
	return Ok(Node::void());
}


pub fn evaluate<'a> (mut env: Env, tree: Node) -> Result<Node, Err<'a>> {
	env.trace.push((*tree.signature.clone(), tree.span));
    let result = match tree.id() {
        (_, "OperatorExpression") => oper_expr_eval(env.clone(), tree)?,
        ("_", "Field") => evaluate_scope(env.clone(), env.data[env.depth].clone(), tree)?,
        ("_", "List") =>  evaluate_collection(env.clone(), *tree.branches[0].clone())?,
        ("_", "Parenthesis") =>  *evaluate_collection(env.clone(), *tree.branches[0].clone())?.branches[0].branches[0].clone(),
        ("_", "Call") =>  call_eval(env.clone(), tree)?,
        ("_", "IfExpression") =>  if_eval(env.clone(), tree)?,
        (_, "LoopExpression") =>  loop_eval(env.clone(), tree)?,
        ("_", "Combinator") => {
        	return Err(Err::new(ErrorClass::CustomError(format!("Cannot invoke '{}' combinator, no target specified", tree.branches[0].into_string()?)), tree));
        },
        ("[!]", "Symbol") => env.data[env.depth].as_node(Node::symbol("<ident>".to_string())),
        (_, "Symbol") => {
        	if let Ok(var) = env.import.get(tree.clone()) { var }else { env.data[env.depth].get(tree)? } // check imported first
        },
        _ => tree
	};
	env.trace.pop();
	Ok(result)
}


fn oper_expr_eval<'a> (env: Env, tree: Node) -> Result<Node, Err<'a>> {
	// <- and : operators cannot act on literals, evaluation must be handled seperately
    if ("<-", "OperatorExpression") == tree.id() {
    	let value = evaluate(env.clone(), *tree.branches[1].clone())?;
    	return env.data[env.depth].set(tree.branches[0].validate_type("Symbol")?, value);
    }
    if (":", "OperatorExpression") == tree.id() { return object_expr_eval(env.clone(), tree); }

    let l_op = evaluate(env.clone(), *tree.branches[0].clone())?;
    let r_op = evaluate(env.clone(), *tree.branches[1].clone())?;
    let oper = if let (op, "OperatorExpression") = tree.id() { op } else { "_" };
    
    match (l_op.get_type(), r_op.get_type()) {
        ("Number", "Number") => {
        	let num1 = l_op.into_number()?;
        	let num2 = r_op.into_number()?;
            match oper {
                "+" => Ok(Node::number(num1+num2)),
                "-" => Ok(Node::number(num1-num2)),
                "*" => Ok(Node::number(num1*num2)),
                "/" => Ok(Node::number(num1/num2)),
                "%" => Ok(Node::number(num1%num2)),
                "^" => Ok(Node::number(num1.powf(num2))),
                
                ">" => Ok(Node::boolean(num1>num2)),
                "<" => Ok(Node::boolean(num1<num2)),
                "=" => Ok(Node::boolean(num1==num2)),
                _ => Err(Err::new(ErrorClass::UndefinedOperation(oper.to_string(), "Number", "Number"), tree.clone()))
        }},
        ("String", "String") => {
        	let str1 = l_op.into_string()?;
        	let str2 = r_op.into_string()?;
        	match oper {
        		"=" => Ok(Node::boolean(str1 == str2)),
        		_ => Err(Err::new(ErrorClass::UndefinedOperation(oper.to_string(), "String", "String"), tree.clone()))
        }},
        // operation between list and any other type
        ("List", _) | (_, "List") => {
        	let (list, other) = match l_op.get_type() {
        		"List" => (l_op, r_op),
        		_ => (r_op, l_op)
        	};
        	
        	match oper {
        		"=" => Ok(Node::boolean(list==other)),
        		"+" => {
        			let mut args = list.branches[0].branches.clone();
        			args.push(Box::new(other));
        			Ok(Node::new(NodeClass::List, [0, 0], vec![Node::new_boxed(NodeClass::Field, [0, 0], args)]))
        		},
        		_ => Err(Err::new(ErrorClass::UndefinedOperation(oper.to_string(), "List", other.get_type()), tree.clone()))
        }},
        (l, r) => Err(Err::new(ErrorClass::UndefinedOperation(oper.to_string(), l, r), tree.clone()))
	}
}


fn evaluate_collection<'a> (env: Env, tree: Node) -> Result<Node, Err<'a>> {
    let mut evaluated: Vec<Node> = Vec::new();
    for branch in tree.branches.iter() { evaluated.push(evaluate(env.clone(), *branch.clone())?); }
    Ok(Node::new(NodeClass::List, tree.span, vec![Node::new(NodeClass::Field, [0, 0], evaluated)]))
}


fn evaluate_scope<'a> (mut env: Env, scope: NameSpace, tree: Node) -> Result<Node, Err<'a>> {
	let mut expr = Ok(Node::void());
    env.depth += 1;
    env.data.push(scope); 
    for branch in tree.branches.iter() {
    	expr = evaluate(env.clone(), *branch.clone());
    	match expr {
    		Err(_) => { break; },
    		Ok(_) => ()
    }}
    env.depth -= 1;
    env.data.pop();
    expr
}


fn object_expr_eval<'a> (mut env: Env, tree: Node) -> Result<Node, Err<'a>> {
	let expr = *tree.branches[1].clone();
	let operand = evaluate(env.clone(), *tree.branches[0].clone()); // result type
	
	if expr.id() == ("_", "Combinator") {
		let object = if ("expect", "Symbol") == expr.branches[0].id() { return handle_error(env.clone(), operand, expr.clone()); }else { operand? };
		
		env.data.push(env.data[env.depth].clone());
		env.depth += 1;  // prevent binding from leaking into outer scope
		
		let bind = expr.branches[1].branches[0].validate_type("Symbol")?;
		env.data[env.depth].set(bind, object)?;
		let call = Node::new_boxed(NodeClass::Call, expr.span, expr.branches[2].branches.clone());
		println!("{}", call.show());
		let result = call_eval(env.clone(), call);

		env.depth -= 1;
		env.data.pop();
		return result;
	}
	
	let object = operand?;
	
	match object.id() {
		("_", "List") => {
			let slice = *evaluate(env.clone(), expr)?.branches[0].clone();
			let array = *object.branches[0].clone();
			match slice.branches.len() {
				1 => {
					let index = slice.branches[0].into_number()?;
					if index as usize >= array.branches.len() { return Err(Err::new(ErrorClass::IndexError(index, object.decode()), object)); }
					return Ok(*array.branches[index as usize].clone());
				},
				2 => {
					let start = array.branches[0].into_number()?;
					let stop = array.branches[1].into_number()?;

					if start > stop { return Err(Err::new(ErrorClass::IndexError(start, object.decode()), object)); }
					if start < 0.0 { return Err(Err::new(ErrorClass::IndexError(start, object.decode()), object)); }
					if stop as usize >= object.branches.len() { return Err(Err::new(ErrorClass::IndexError(stop, object.decode()), object)); }

					return Ok(Node::new_boxed(NodeClass::List, tree.span, array.branches[start as usize..stop as usize].to_vec()));
				},
				_ => { return Err(Err::new(ErrorClass::IndexError(2.0, object.decode()), slice)); }
			}
			
		},
		(inst_of, "ObjectInstance") => {
			let operations = match *expr.signature {
				NodeClass::Field => expr,
				_ => Node::new(NodeClass::Field, expr.span, vec![expr])
			};
			let mut result = Ok(Node::void());
			let target_ns = if inst_of == "<extern_link>" { env.data[object.branches[0].into_number()? as usize].clone() }
			else { NameSpace::from(*object.branches[0].clone(), *object.branches[1].clone())? };
			
			// operations on object instances are allowed to access global variables
			let extern_keys = env.data[env.depth].keys.read().unwrap().clone();
			let extern_vals = env.data[env.depth].values.read().unwrap().clone();
			target_ns.keys.write().unwrap().extend(extern_keys);
			target_ns.values.write().unwrap().extend(extern_vals);
			
			result = evaluate_scope(env.clone(), target_ns, operations);
			// I could use .unwrap_or_else() but this way feels more intuitive to me
			match result {
				Ok(ref val) => { 
					if ("<ident>", "ObjectInstance") == val.id() { result = Ok(Node::new_boxed(NodeClass::ObjectInst(inst_of.to_string()), val.span, val.branches.clone())); }
				},
				Err(ref e) => {
					if *e.class == ErrorClass::Signal("StopFunction") { result = Ok(e.cause.clone()); }
			}}
			return result;
		},
		(_, _) => Err(Err::new(ErrorClass::UndefinedOperation(":".to_string(), object.get_type(), expr.get_type()), object))
	}
}


fn call_eval<'a> (env: Env, tree: Node) -> Result<Node, Err<'a>> {
	let stdlib_search = search_library(env.clone(), tree.clone())?;
	let mut object_return = Ok(Node::void());
	
	if stdlib_search.id() != ("_", "Void") { object_return = Ok(stdlib_search); }
	else {
		let object = evaluate(env.clone(), *tree.branches[0].clone())?.validate_type("Object")?;
		let arguments = evaluate_collection(env.clone(), *tree.branches[1].clone())?;
		let func_ns = NameSpace::from(*object.branches[0].clone(), *arguments.branches[0].clone())?;
		// create link to function scope allowing inner methods to access it
		func_ns.set(Node::symbol("[@]".to_string()), Node::new(NodeClass::ObjectInst("<extern_link>".to_string()), [0, 0], vec![Node::number(env.depth as f32)]))?;
		object_return = evaluate_scope(env.clone(), func_ns, *object.branches[1].clone());
	}
    
    match object_return {
    	Ok(ref val) => { 
    		if ("<ident>", "ObjectInstance") == val.id() { // set instance's name to caller's name
				let inst_of = if let (label, "Symbol") = tree.branches[0].id() { label.to_string() }else { "<anon>".to_string() };
				return Ok(Node::new_boxed(NodeClass::ObjectInst(inst_of), val.span, val.branches.clone()));
			}
			else { return object_return; }
		},
		Err(ref e) => { 
			if ErrorClass::Signal("StopFunction") == *e.class { return Ok(e.cause.clone()); }
			else { return object_return; }
	}}
}


fn if_eval<'a> (env: Env, tree: Node) -> Result<Node, Err<'a>> {
    for if_stat in tree.branches.iter() {
        if evaluate(env.clone(), *if_stat.branches[0].clone())?.into_boolean()? {
			return evaluate_scope(env.clone(), env.data[env.depth].clone(), *if_stat.branches[1].clone()); 
		}
        else if if_stat.branches.len() == 3 {
        	return evaluate_scope(env.clone(), env.data[env.depth].clone(), *if_stat.branches[2].clone());
    }}
    Ok(Node::void())
}


fn loop_eval<'a> (env: Env, tree: Node) -> Result<Node, Err<'a>> {
    match tree.id() {
        ("cond", "LoopExpression") => {
            let condition = *tree.branches[0].branches[0].clone();
            let contents = *tree.branches[1].clone();
            
            let mut comp: Vec<Node> = Vec::new();
            let mut temp = Ok(Node::void());
            while evaluate(env.clone(), condition.clone())?.into_boolean()? {
                temp = evaluate_scope(env.clone(), env.data[env.depth].clone(), contents.clone());
                match temp {
                	Err(ref e) => { 
                		if ErrorClass::Signal("StopIteration") == *e.class { break; }
                		if ErrorClass::Signal("ResetIteration") == *e.class { continue; }
                		else { return temp; }
                	},
                	Ok(val) => { comp.push(val); }
            }}
        	Ok(Node::new(NodeClass::List, tree.span, vec![Node::new(NodeClass::Field, tree.span, comp)]))
        },
        ("iter", "LoopExpression") => {
            let index = *tree.branches[0].branches[0].clone(); // name taken by each index
            let iterator = evaluate(env.clone(), *tree.branches[0].branches[1].clone())?.validate_type("List")?.branches[0].clone();
            let contents = match tree.branches.len() {
            	2 => *tree.branches.last().unwrap().clone(),
            	_ => Node::new(NodeClass::Field, tree.span, vec![Node::new_boxed(NodeClass::Loop("iter".to_string()), tree.span, tree.branches[1..].to_vec())])
            };
            let mut comp: Vec<Node> = Vec::new();
            let mut temp = Ok(Node::void());
            
            let loop_scope = env.data[env.depth].clone();
            for elem in iterator.branches.iter() {
            	loop_scope.set(index.clone(), *elem.clone())?;
            	temp = evaluate_scope(env.clone(), loop_scope.clone(), contents.clone());
                match temp {
                	Err(ref e) => { 
						if ErrorClass::Signal("StopIteration") == *e.class { break; }
						if ErrorClass::Signal("ResetIteration") == *e.class { continue; }
						else { return temp; }
                	},
                	Ok(val) => { comp.push(val); }
            }}
            Ok(Node::new(NodeClass::List, tree.span, vec![Node::new(NodeClass::Field, [0, 0], comp)]))
        },
        _ => Ok(tree)
	}
}
