/*
ToDo:
Implement special characters '~' (literal field) and '\' (list of fields)
*/

use std::{fs, io, path::Path, str::Chars};

enum KeysightPrefix {
    //{@A-CAP|test status|measured value|subtest designator}
    ACap(i32, f32, Option<String>),

    UserDefined(Vec<String>),
    Error(Vec<String>),
}

fn to_int(field: Option<&String>) -> Option<i32> {
    if let Some(string) = field {
        if let Ok(i) = string.parse::<i32>() {
            return Some(i);
        }
    }

    None
}

fn to_float(field: Option<&String>) -> Option<f32> {
    if let Some(string) = field {
        if let Ok(i) = string.parse::<f32>() {
            return Some(i);
        }
    }

    None
}

impl KeysightPrefix {
    fn new(data: Vec<String>) -> Option<Self> {
        if let Some(first) = data.first() {
            match first.as_str() {
                //{@A-CAP|test status|measured value|subtest designator}
                "@A-CAP" => {
                    let status = to_int(data.get(1));
                    let result = to_float(data.get(2));
                    if status.is_some() && result.is_some() {
                        let designator = {
                            if data.len() > 3 {
                                Some(data[3].clone())
                            } else {
                                None
                            }
                        };

                        Some(KeysightPrefix::ACap(
                            status.unwrap(),
                            result.unwrap(),
                            designator,
                        ))
                    } else {
                        Some(KeysightPrefix::Error(data.clone()))
                    }
                }

                _ => Some(KeysightPrefix::UserDefined(data.clone())),
            }
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct TreeNode {
    data: Vec<String>,
    branches: Vec<TreeNode>,
}

impl TreeNode {
    fn read(buffer: &mut Chars) -> Self {
        let mut branches: Vec<TreeNode> = Vec::new();
        let mut data_buff: String = String::new();

        loop {
            let c = buffer.next();
            if c.is_none() || c.is_some_and(|f| f == '}') {
                break;
            }

            let c = c.unwrap();
            if c == '{' {
                branches.push(TreeNode::read(buffer));
            } else if c != '\n' {
                data_buff.push(c);
            }
        }

        let data = data_buff.split('|').map(|f| f.to_string()).collect();

        TreeNode { data, branches }
    }

    pub fn interpret(&self) {}
}

pub fn parse_file(path: &Path) -> io::Result<Vec<TreeNode>> {
    let file = fs::read_to_string(path)?;
    let mut buffer = file.chars();

    let mut tree: Vec<TreeNode> = Vec::new();
    loop {
        let c = buffer.next();
        if c.is_none() {
            break;
        }

        tree.push(TreeNode::read(&mut buffer));
    }

    Ok(tree)
}
