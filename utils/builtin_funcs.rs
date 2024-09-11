/*
functions included here:

-------- vector/ matrix operations
product
dot_product
transpose
determinant
adjoint
invert
span

-------- set operations
intersect
different
contains
maximum
minimum

-------- type conversion
to_int
to_string

-------- core utils
write
reset
stop
out
type
ext
prompt

*/

// I would normally use std::collections::HashSet with a.intersect(&b), but f32 is not hashable

use regex::Regex; // used for bool and int type conversion
use std::io::Write; // used by prompt and write functions
use std::io;
use crate::utils::node::{ NodeClass, Node };
use crate::utils::error::{ ErrorClass, Err };
use crate::evaluator::{ Env, evaluate, load_file };


fn intersect<'a> (env: Env, tree: Node) -> Result<Node, Err<'a>> {
	let a = tree.branches[0].validate_type("List")?.branches[0].branches.clone();
	let b = tree.branches[1].validate_type("List")?.branches[0].branches.clone();
	let mut similar = Vec::new();
	for elem_a in a.into_iter() {
		match b.iter().find(|elem_b| elem_a == **elem_b) {
			Some(num) => { similar.push(num.clone()); },
			None => ()
	}}
	let r = Node::new(NodeClass::List, [0, 0], vec![Node::new_boxed(NodeClass::Field, [0, 0], similar)]);
	return Ok(r);
}


fn length<'a> (env: Env, tree: Node) -> Result<Node, Err<'a>> {
	let len = tree.branches[0].validate_type("List")?.branches[0].branches.len();
	return Ok(Node::number(len as f32));
}


fn contains<'a> (env: Env, tree: Node) -> Result<Node, Err<'a>> {
	let search = tree.branches[0].clone();
	let target = tree.branches[1].validate_type("List")?.branches[0].branches.clone();
	
	match target.into_iter().find(|s| search == *s) {
		Some(_) => { return Ok(Node::boolean(true)); },
		None => { return Ok(Node::boolean(false)); }
	}
}


fn span<'a> (env: Env, tree: Node) -> Result<Node, Err<'a>> {
	// get first element from each array
	let arr1 = tree.branches[0].validate_type("List")?.branches[0].clone();
	let arr2 = tree.branches[1].validate_type("List")?.branches[0].clone();
	
	if arr1.branches.len() != arr2.branches.len() { return Err(Err::new(ErrorClass::IndexError(arr2.branches.len() as f32, arr1.decode()), tree)); }
	let (x1, x2) = (arr1.branches[0].into_number()? as usize, arr2.branches[0].into_number()? as usize);
	let range_x: Vec<Node> = (x1..x2).map(|val| Node::number(val as f32)).collect();
	
	// base case
	if arr1.branches.len() == 1 { return Ok(Node::new(NodeClass::List, [0, 0], vec![Node::new(NodeClass::Field, [0, 0], range_x)])); }
	
	let inner1 = Node::new(NodeClass::List, [0, 0], vec![Node::new_boxed(NodeClass::Field, [0, 0], tree.branches[0].branches[0].branches[1..].to_vec())]);
	let inner2 = Node::new(NodeClass::List, [0, 0], vec![Node::new_boxed(NodeClass::Field, [0, 0], tree.branches[1].branches[0].branches[1..].to_vec())]);
	
	// first layer
	let range_y = span(env.clone(), Node::new(NodeClass::Field, [0, 0], vec![inner1, inner2]))?;
	
	let mut result = Vec::new();
	for x in range_x.into_iter() {
		for y in range_y.branches[0].branches.clone().into_iter() {
			let mut elem = vec![Box::new(x.clone())];
			if ("_", "List") == y.id() { elem.extend(y.branches[0].branches.clone()); }
			else { elem.push(y); }
			result.push(Node::new(NodeClass::List, [0, 0], vec![Node::new_boxed(NodeClass::Field, [0, 0], elem)]));
	}}
	
	return Ok(Node::new(NodeClass::List, [0, 0], vec![Node::new(NodeClass::Field, [0, 0], result)]));
}


fn to_num<'a> (env: Env, tree: Node) -> Result<Node, Err<'a>> {
	let target = *tree.branches[0].clone();
	match *target.signature {
		NodeClass::Number(num) => Ok(target.clone()),
		NodeClass::Boolean(b) => if b { Ok(Node::number(1.0)) }else { Ok(Node::number(0.0)) },
		NodeClass::String(ref val) => {
			let pattern = Regex::new(r"^\-?[0-9]+\.?[0-9]*$").unwrap(); // simple regex for recognizing floating points
			match pattern.find(val) {
				Some(num) => Ok(Node::number(num.as_str().parse::<f32>().unwrap())),
				None => Err(Err::new(ErrorClass::Conversion(format!("{}", val), "String", "Number"), tree))
		}},
		_ => Err(Err::new(ErrorClass::Conversion(target.decode(), target.get_type(), "Number"), tree))
	}
}


fn reset<'a> (env: Env, tree: Node) -> Result<Node, Err<'a>> { return Err(Err::new(ErrorClass::Signal("ResetIteration"), tree)); }


fn stop<'a> (env: Env, tree: Node) -> Result<Node, Err<'a>> { return  Err(Err::new(ErrorClass::Signal("StopIteration"), *tree.branches[0].clone())); }


fn out<'a> (env: Env, tree: Node) -> Result<Node, Err<'a>> { return Err(Err::new(ErrorClass::Signal("StopFunction"), *tree.branches[0].clone())); }


fn class<'a> (env: Env, tree: Node) -> Result<Node, Err<'a>> { return Ok(Node::new(NodeClass::String(tree.branches[0].get_type().to_string()), [0, 0], Vec::new())); }


fn write<'a> (env: Env, tree: Node) -> Result<Node, Err<'a>> {
    for string in tree.branches.iter() { 
    	match string.id() {
    		("$n", "String") => { print!("\n"); },
    		(s, "String") => { print!("{}", s); },
    		_ => { print!("{}", string.decode()); }
    }}
    io::stdout().flush().unwrap();
    return Ok(tree);
}


fn prompt<'a> (env: Env, tree: Node) -> Result<Node, Err<'a>> {
	let prompt = tree.branches[0].into_string()?;
	let mut command = String::new();
	print!("{}", prompt);
	io::stdout().flush().unwrap();
	io::stdin().read_line(&mut command).unwrap();
	return Ok(Node::new(NodeClass::String(command.trim().to_string()), [0, 0], Vec::new()));
}


/*
planning to make this a struct StandardLibrary with fields:
- func_library: HashMap<String, Box<dyn FnMut<'a>(Env, Node) -> Result<Node, Err<'a>>>>
and methods:
- fn create () -> StandardLibrary  (new stdlib with default functions)
- fn search (env: Env, query: Node) -> Result<Node, Err>>>
- fn add_func (func: Box<dyn FnMut(Env, Node) -> Result<Node, Err>>) -> Result<(), Err>
- fn rem_func (func_name: String) -> Result<(), Err>
*/
pub fn search_library<'a> (env: Env, search: Node) -> Result<Node, Err<'a>> {
	let name: &str = if let (n, "Symbol") = search.branches[0].id() { n }else { return Ok(Node::void()); };
	
	let mut evaluated = Vec::new();
	for branch in search.branches[1].branches.iter() { evaluated.push(evaluate(env.clone(), *branch.clone())?); }
	let arguments = Node::new(NodeClass::Field, [0, 0], evaluated);

	match name {
		"write" => write(env, arguments),
		"prompt" => prompt(env, arguments.validate_args_len(1)?),
		"out" => out(env, arguments.validate_args_len(1)?),
		"type" => class(env, arguments.validate_args_len(1)?),
		"stop" => stop(env, arguments.validate_args_len(1)?),
		"reset" => reset(env, arguments.validate_args_len(0)?),
		
		"span" => span(env, arguments.validate_args_len(2)?),
		"toNumber" => to_num(env, arguments.validate_args_len(1)?),
		"intersect" => intersect(env, arguments.validate_args_len(2)?),
		"len" => length(env, arguments.validate_args_len(1)?),
		"in" => contains(env, arguments.validate_args_len(2)?),
		_ => Ok(Node::void())
	}
}

