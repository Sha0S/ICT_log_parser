/*
ToDo:
Implement special characters '~' (literal field) and '\' (list of fields)
*/

use std::{fs, io, path::Path, str::Chars};

pub enum AnalogTest {
    Cap,         // A-CAP
    Diode,       // A-DIO
    Fuse,        // A-FUS
    Inductor,    // A-IND
    Jumper,      // A-JUM
    Measurement, // A-MEA
    NFet,        // A-NFE
    PFet,        // A-PFE
    Npn,         // A-NPN
    Pnp,         // A-PNP
    Pot,         // A-POT
    Res,         // A-RES
    Switch,      // A-SWI
    Zener,       // A-ZEN

    Error,
}

impl From<&str> for AnalogTest {
    fn from(value: &str) -> Self {
        match value {
            "@A-CAP" => Self::Cap,
            "@A-DIO" => Self::Diode,
            "@A-FUS" => Self::Fuse,
            "@A-IND" => Self::Inductor,
            "@A-JUM" => Self::Jumper,
            "@A-MEA" => Self::Measurement,
            "@A-NFE" => Self::NFet,
            "@A-PFE" => Self::PFet,
            "@A-NPN" => Self::Npn,
            "@A-PNP" => Self::Pnp,
            "@A-POT" => Self::Pot,
            "@A-RES" => Self::Res,
            "@A-SWI" => Self::Switch,
            "@A-ZEN" => Self::Zener,

            _ => Self::Error,
        }
    }
}

pub enum KeysightPrefix {
    // {@A-???|test status|measured value|subtest designator}
    Analog(AnalogTest, i32, f32, Option<String>),

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

fn get_string(data: &[String], index: usize) -> Option<String> {
    if data.len() >= index {
        Some(data[index].clone())
    } else {
        None
    }
}

impl KeysightPrefix {
    fn new(data: Vec<String>) -> Option<Self> {
        if let Some(first) = data.first() {
            match first.as_str() {
                // {@A-???|test status|measured value|subtest designator} designator is optional
                "@A-CAP" | "@A-DIO" | "@A-FUS" | "@A-IND" | "@A-JUM" | "@A-MEA" | "@A-NFE"
                | "@A-PFE" | "@A-NPN" | "@A-PNP" | "@A-POT" | "@A-RES" | "@A-SWI" | "@A-ZEN" => {
                    let status = to_int(data.get(1));
                    let result = to_float(data.get(2));
                    if let Some(s) = status {
                        if let Some(r) = result {
                            return Some(KeysightPrefix::Analog(
                                data[0].as_str().into(),
                                s,
                                r,
                                get_string(&data, 3),
                            ));
                        }
                    }
                    Some(KeysightPrefix::Error(data.clone()))
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
