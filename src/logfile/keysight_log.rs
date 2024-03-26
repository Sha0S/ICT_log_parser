/*
ToDo:
Implement special characters '~' (literal field) and '\' (list of fields)

Q:
- BATCH -> "version label" field?
*/

use std::{fs, io, path::Path, str::Chars};

type Result<T> = std::result::Result<T, ParsingError>;

#[derive(Debug, Clone)]
struct ParsingError;

#[derive(Debug)]
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

#[derive(Debug)]
pub enum KeysightPrefix {
    // {@A-???|test status|measured value (|subtest designator)}
    Analog(AnalogTest, i32, f32, Option<String>),

    // {@AID|time detected|serial number}
    AlarmId(u64, String),
    // {@ALM|alarm type|alarm status|time detected|board type|board type rev|alarm limit|detected value|controller|testhead number}
    Alarm(i32, bool, u64, String, String, i32, i32, String, i32),

    // {@ARRAY|subtest designator|status|failure count|samples}
    Array(String, i32, i32, i32),

    // {@BATCH|UUT type|UUT type rev|fixture id|testhead number|testhead type|process step|batch id|
    //      operator id|controller|testplan id|testplan rev|parent panel type|parent panel type rev (| version label)}
    Batch(
        String,
        String,
        i32,
        i32,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        Option<String>,
    ),

    // {@BLOCK|block designator|block status}
    Block(String, i32),

    // {@BS-CON|test designator|status|shorts count|opens count}
    Boundary(String, i32, i32, i32),
    // {@BS-O|first device|first pin|second device|second pin}
    BoundaryOpen(String, i32, String, i32),
    // {@BS-S|cause|node list}
    BoundaryShort(String, String),

    UserDefined(Vec<String>),
    Error(String),
}

fn to_int(field: Option<&String>) -> Result<i32> {
    if let Some(string) = field {
        if let Ok(i) = string.parse::<i32>() {
            return Ok(i);
        }
    }

    Err(ParsingError)
}

fn to_uint(field: Option<&String>) -> Result<u64> {
    if let Some(string) = field {
        if let Ok(i) = string.parse::<u64>() {
            return Ok(i);
        }
    }

    Err(ParsingError)
}

fn to_float(field: Option<&String>) -> Result<f32> {
    if let Some(string) = field {
        if let Ok(i) = string.parse::<f32>() {
            return Ok(i);
        }
    }

    Err(ParsingError)
}

fn to_bool(field: Option<&String>) -> Result<bool> {
    if let Some(string) = field {
        match string.as_str() {
            "0" => return Ok(false),
            "1" => return Ok(true),
            _ => return Err(ParsingError),
        }
    }

    Err(ParsingError)
}

fn get_string(data: &[String], index: usize) -> Option<String> {
    if data.len() > index {
        Some(data[index].clone())
    } else {
        None
    }
}

impl KeysightPrefix {
    fn new(data: Vec<String>) -> Result<Self> {
        if let Some(first) = data.first() {
            match first.as_str() {
                // {@A-???|test status|measured value (|subtest designator)}
                "@A-CAP" | "@A-DIO" | "@A-FUS" | "@A-IND" | "@A-JUM" | "@A-MEA" | "@A-NFE"
                | "@A-PFE" | "@A-NPN" | "@A-PNP" | "@A-POT" | "@A-RES" | "@A-SWI" | "@A-ZEN" => {
                    Ok(KeysightPrefix::Analog(
                        data[0].as_str().into(),
                        to_int(data.get(1))?,
                        to_float(data.get(2))?,
                        get_string(&data, 3),
                    ))
                }

                // {@AID|time detected|serial number}
                "@AID" => {
                    if let Some(serial) = get_string(&data, 2) {
                        Ok(KeysightPrefix::AlarmId(to_uint(data.get(1))?, serial))
                    } else {
                        Err(ParsingError)
                    }
                }
                // {@ALM|alarm type|alarm status|time detected|board type|board type rev|alarm limit|detected value|controller|testhead number}
                "@ALM" => {
                    if data.len() < 10 {
                        return Err(ParsingError);
                    }

                    Ok(KeysightPrefix::Alarm(
                        to_int(data.get(1))?,
                        to_bool(data.get(2))?,
                        to_uint(data.get(3))?,
                        get_string(&data, 4).unwrap(),
                        get_string(&data, 5).unwrap(),
                        to_int(data.get(6))?,
                        to_int(data.get(7))?,
                        get_string(&data, 8).unwrap(),
                        to_int(data.get(9))?,
                    ))
                }

                // {@ARRAY|subtest designator|status|failure count|samples}
                "@ARRAY" => {
                    if data.len() < 5 {
                        return Err(ParsingError);
                    }

                    Ok(KeysightPrefix::Array(
                        get_string(&data, 1).unwrap(),
                        to_int(data.get(2))?,
                        to_int(data.get(3))?,
                        to_int(data.get(4))?,
                    ))
                }

                // {@BATCH|UUT type|UUT type rev|fixture id|testhead number|testhead type|process step|batch id|
                //      operator id|controller|testplan id|testplan rev|parent panel type|parent panel type rev (| version label)}
                "@BATCH" => {
                    if data.len() < 14 {
                        return Err(ParsingError);
                    }

                    Ok(KeysightPrefix::Batch(
                        get_string(&data, 1).unwrap(),
                        get_string(&data, 2).unwrap(),
                        to_int(data.get(3))?,
                        to_int(data.get(4))?,
                        get_string(&data, 5).unwrap(),
                        get_string(&data, 6).unwrap(),
                        get_string(&data, 7).unwrap(),
                        get_string(&data, 8).unwrap(),
                        get_string(&data, 9).unwrap(),
                        get_string(&data, 10).unwrap(),
                        get_string(&data, 11).unwrap(),
                        get_string(&data, 12).unwrap(),
                        get_string(&data, 13).unwrap(),
                        get_string(&data, 14),
                    ))
                }

                // {@BLOCK|block designator|block status}
                "@BLOCK" => {
                    if data.len() < 3 {
                        return Err(ParsingError);
                    }

                    Ok(KeysightPrefix::Block(
                        get_string(&data, 1).unwrap(),
                        to_int(data.get(2))?,
                    ))
                }

                // {@BS-CON|test designator|status|shorts count|opens count}
                "@BS-CON" => {
                    if data.len() < 5 {
                        return Err(ParsingError);
                    }

                    Ok(KeysightPrefix::Boundary(
                        get_string(&data, 1).unwrap(),
                        to_int(data.get(2))?,
                        to_int(data.get(3))?,
                        to_int(data.get(4))?,
                    ))
                }
                // {@BS-O|first device|first pin|second device|second pin}
                "@BS-O" => {
                    if data.len() < 5 {
                        return Err(ParsingError);
                    }

                    Ok(KeysightPrefix::BoundaryOpen(
                        get_string(&data, 1).unwrap(),
                        to_int(data.get(2))?,
                        get_string(&data, 3).unwrap(),
                        to_int(data.get(4))?,
                    ))
                }
                // {@BS-S|cause|node list}
                "@BS-S" => {
                    if data.len() < 3 {
                        return Err(ParsingError);
                    }

                    Ok(KeysightPrefix::BoundaryShort(
                        get_string(&data, 1).unwrap(),
                        get_string(&data, 2).unwrap(),
                    ))
                }

                _ => Ok(KeysightPrefix::UserDefined(data)),
            }
        } else {
            Err(ParsingError)
        }
    }
}

#[derive(Debug)]
pub struct TreeNode {
    data: KeysightPrefix,
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

        if let Ok(data) = KeysightPrefix::new(data_buff.split('|').map(|f| f.to_string()).collect())
        {
            TreeNode { data, branches }
        } else {
            TreeNode {
                data: KeysightPrefix::Error(data_buff),
                branches,
            }
        }
    }

    fn print(&self, indent: i32) {
        for _ in 0..indent {
            print!("\t");
        }

        println!("{:?}", self.data);

        for b in &self.branches {
            b.print(indent + 1);
        }
    }
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
