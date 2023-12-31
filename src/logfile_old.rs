#![allow(dead_code)]

use std::fs;
use std::path::Path;
use std::time::SystemTime;

use umya_spreadsheet::*;

/* Conversions */

fn str_to_result(s: &str) -> bool {
    match s {
        "0" => true,
        "00" => true,
        _ => false
    }
}


fn result_to_str(r: bool) -> String {
    match r {
        true => "Pass".to_string(),
        false => "Fail".to_string()
    }
}

fn time_to_str(t: String) -> String {
    format!("{}/{}/{} {}:{}:{}", 
    &t[0..2], &t[2..4], &t[4..6],
    &t[6..8], &t[8..10], &t[10..])
}

/* TType Start */

#[derive(Clone)]
enum TType {
    Pin,
    Shorts,
    Jumper,
    Resistor,
    Capacitor,
    Inductor,
    Diode,
    Zener,
    Testjet,
    Digital,
    Measurement,
    Unknown (String)
}

impl TType {
    fn new(s: &str) -> Self {
        match s {
            "PF" => TType::Pin,
            "TS" => TType::Shorts,
            "A-JUM" => TType::Jumper,
            "A-RES" => TType::Resistor,
            "A-CAP" => TType::Capacitor,
            "A-DIO" => TType::Diode,
            "A-ZEN" => TType::Zener,
            "A-IND" => TType::Inductor,
            "TJET" => TType::Testjet,
            "D-T" => TType::Digital,
            "A-MEA" => TType::Measurement,
    
            _ => {
                println!("{}",s);
                TType::Unknown (s.to_string()) }
        }
    }

    fn print(&self) -> String {
        match self {
            TType::Pin => "Pin".to_string(),
            TType::Shorts => "Shorts".to_string(),
            TType::Jumper => "Jumper".to_string(),
            TType::Resistor => "Resistor".to_string(),
            TType::Capacitor => "Capacitor".to_string(),
            TType::Inductor => "Inductor".to_string(),
            TType::Diode => "Diode".to_string(),
            TType::Zener => "Zener".to_string(),
            TType::Testjet => "Testjet".to_string(),
            TType::Digital => "Digital".to_string(),
            TType::Measurement => "Measurement".to_string(),
            TType::Unknown (x) => x.to_string()
        }
    }
}

/* TType End */
/* TLimit Start */

#[derive(Debug, Clone, PartialEq)]
enum TLimit {
    None,
    Lim2 (String,String), // UL, LL
    Lim3 (String,String,String) // Nom, UL, LL
}

/* TLimit End */
/* Test Start */

#[derive(Clone)]
struct Test {
    name: String,
    ttype: TType,
    result: bool,
    measurement: String,

    limits: TLimit
}


impl Test {
    fn save(&self, file: &mut Spreadsheet, line: u32){
        file.get_sheet_mut(&0).unwrap().get_cell_mut((1,line)).set_value(self.name.clone());
        file.get_sheet_mut(&0).unwrap().get_cell_mut((2,line)).set_value(self.ttype.print());
        file.get_sheet_mut(&0).unwrap().get_cell_mut((3,line)).set_value(result_to_str(self.result));
        file.get_sheet_mut(&0).unwrap().get_cell_mut((4,line)).set_value(self.measurement.clone());

        match self.limits.clone() {
            TLimit::Lim3 (nom,ul,ll) => {
                file.get_sheet_mut(&0).unwrap().get_cell_mut((5,line)).set_value(ll);
                file.get_sheet_mut(&0).unwrap().get_cell_mut((6,line)).set_value(nom);
                file.get_sheet_mut(&0).unwrap().get_cell_mut((7,line)).set_value(ul);
            }
            TLimit::Lim2 (ul,ll) => {
                file.get_sheet_mut(&0).unwrap().get_cell_mut((5,line)).set_value(ll);
                file.get_sheet_mut(&0).unwrap().get_cell_mut((7,line)).set_value(ul);
            }
            TLimit::None => {}
        }
    }
}

/* Test End */
/* LogFile Start */

#[derive(Clone)]
pub struct Logfile {
    source: std::path::PathBuf,
    DMC: String,
    DMC_master: String,
    product_name: String,
    time: String,
    result: bool,
    tests: Vec<Test>
}

impl Logfile {
    pub fn save(&self, dir: &Path) {
        let mut p:std::path::PathBuf = dir.to_path_buf();
        if p.is_dir() {
            let mut filen = self.DMC.clone();
            filen = filen + " " + self.time.as_str() + ".xlsx";
            p = p.join(Path::new(&filen));
        }

        let mut f = new_file();
        f.get_sheet_mut(&0).unwrap().get_cell_mut((2,1)).set_value(self.source.to_str().unwrap());
        f.get_sheet_mut(&0).unwrap().get_cell_mut((1,1)).set_value(result_to_str(self.result));

        let mut l:u32 = 2;
        for test in &self.tests {
            test.save(&mut f, l);
            l = l+1;
        }
        let _ = writer::xlsx::write(&f, p);
    }

    pub fn load(p: &Path) -> Self {
        //println!("INFO: LF.load started");
        let mut DMC = String::new();
        let mut DMC_master = String::new();
        let mut product_name = String::new();
        let mut time = String::new();
        let mut result = false;
        let mut tests = Vec::new();

        let fileb = fs::read_to_string(p).unwrap();
        let mut lines = fileb.lines();
        let mut iline = lines.next();

        //println!("\tINFO: parsing started");
        while iline != None {
            let mut line = iline.unwrap().trim();
            if line == "}}" || line == "}" || line == "" {
                iline = lines.next();
                continue;
            } 

            let mut parts = line.split("|");
            let word = parts.next();

            match word.unwrap() {
                "{@BATCH" => {
                    product_name = parts.next().unwrap().to_string();
                }
                "{@BTEST" => {
                    DMC = parts.next().unwrap().to_string();
                    result = str_to_result(parts.next().unwrap());
                    time = parts.nth(7).unwrap().to_string();
                    DMC_master = parts.nth(2).unwrap().to_string();
                }
                "{@PF" => {
                    // WIP - pin test
                }
                "{@TS" => {
                    // WIP - shorts/opens
                }
                "{@TJET" => {
                    // WIP - testjet
                }
                "{@D-T" => {
                    let tresult = str_to_result(parts.next().unwrap());
                                
                    let test = Test{    
                        name: parts.last().unwrap().to_string(),
                        ttype: TType::Digital,
                        result: tresult,
                        measurement: "".to_string(),
                        limits: TLimit::None};

                    tests.push(test);

                    _ = lines.next();
                }
                "{@RPT" => {
                    // report block -> ignored
                }
                "{@BLOCK" => {
                    let name = parts.next().unwrap().to_string();
                    let _tresult = str_to_result(parts.next().unwrap());
                    let mut dt_counter = 1;

                    iline = lines.next();

                    if name.ends_with("testjet") { continue; }
                    
                    while iline != None {
                        line = iline.unwrap().trim();
                        if line == "}" { break; } 

                        let mut part = line.split("{@");
                        parts = part.nth(1).unwrap().split("|");

                        let ttype = TType::new (parts.next().unwrap());

                        match ttype {
                            TType::Unknown (_) => {
                                /*let _test = Test{    
                                    name: _name.clone(),
                                    ttype: _ttype,
                                    result: _tresult,
                                    measurement: "".to_string(),
                                    limits: TLimit::None};

                                _tests.push(_test);*/
                            }
                            TType::Digital => {

                                let tresult2 = str_to_result(parts.next().unwrap());
                                
                                let _test = Test{    
                                    name: format!("{}:digital_{}",parts.last().unwrap(), dt_counter),
                                    ttype,
                                    result: tresult2,
                                    measurement: "".to_string(),
                                    limits: TLimit::None};

                                tests.push(_test);
                                dt_counter = dt_counter+1;

                                _ = lines.next();
                            }
                            _ => {
                                let mut name2 = name.clone();
                                let tresult2 = str_to_result(parts.next().unwrap());
                                let measurement = parts.next().unwrap().to_string();

                                match parts.next() {
                                    Some(x) => {
                                        name2 = name2 + "%" + x;
                                    }
                                    None => {                                        
                                    }
                                }

                                let limits:TLimit;
                                match part.next() {
                                    Some(x) => {
                                        parts = x.strip_suffix("}}").unwrap().split("|");
                                        match parts.next().unwrap() {
                                            "LIM2" => {
                                                limits = TLimit::Lim2 (
                                                    parts.next().unwrap().to_string(),
                                                    parts.next().unwrap().to_string());
                                            }
                                            "LIM3" => {
                                                limits = TLimit::Lim3 (
                                                    parts.next().unwrap().to_string(),
                                                    parts.next().unwrap().to_string(),
                                                    parts.next().unwrap().to_string());
                                            }
                                            _ => {
                                                limits = TLimit::None;
                                            }
                                        }
                                    }

                                    None => {
                                        limits = TLimit::None;
                                    }
                                }

                                let test = Test{    
                                    name: name2.clone(),
                                    ttype,
                                    result: tresult2,
                                    measurement,
                                    limits};

                                tests.push(test);

                            }
                        }

                        iline = lines.next();
                    }
                }
                _ => {
                    println!("Unable to process {}", word.unwrap())
                }
            }

            iline = lines.next();
        }

        Self {
            source: p.to_path_buf(),
            DMC,
            DMC_master,
            product_name,
            time,
            result,
            tests
        }
    }
}
/* LogFile End */


/* MasterLogfile Start */
#[derive(Default)]
pub struct MasterLogfile {
    product_name: String,
    logs: Vec<Logfile>,
    test_list: Vec<String>,
}

impl MasterLogfile {
    // Add one Logfile to the list from a path
    pub fn push_from_file(&mut self, p: &Path) -> bool {  
        println!("MLF.push_from_file {}", p.display());
        self.push(Logfile::load(p))
    }

    // Add one Logfile to the list, consuming it in the process. Return true in a sucess. 
    pub fn push(&mut self, log: Logfile) -> bool {
        println!("MLF.push - start");
        if self.logs.len() > 0 {
            // Check if it is for the same type.
            // Mismatched types are not supported. (And ATM I see no reason to do that.)
            if log.product_name != self.product_name {
                println!("\tERR: Type mismatch detected! {} =/= {}", self.product_name, log.product_name);
                return false
            }
            
            // Check if the testlist matches
            // WIP
            //  I will have to latter add support for changes in the test order.
            //  Current itteration assumes the logs come from the same ICT, 
            //  and no major chages where made between tests!
            // WIP
            for (t1, t2) in self.test_list.iter().zip(log.tests.iter()) {
                if *t1 != t2.name {
                    println!("\tERR: Test mismatch detected! {} =/= {}", *t1, t2.name);
                    return false
                }
            }
            
            // If the new one has a longer testlist, then extend the current one.
            if self.test_list.len() < log.tests.len() {
                println!("\tINFO: Updating test_list.");
                self.test_list.clear();

                // I'm sure there is a faster way, but we rarely should have to do this, if ever.
                for t in &log.tests {
                    self.test_list.push(t.name.to_owned());
                }
            }

        } else {
            println!("\tINFO: Initializing as {}", log.product_name);
            self.product_name = log.product_name.to_owned();

            self.test_list.clear();
            for t in &log.tests {
                self.test_list.push(t.name.to_owned());
            }
        }

        println!("\tINFO: Pushing the log into the stack.");
        self.logs.push(log);
        println!("MLF.push - success");
        true 
    }
}

struct TResult {
    result: bool,
    measurement: f64
}
struct Log {
    time: SystemTime,
    result: bool,

    results: Vec<TResult>,
    limits: Vec<TLimit>
}
struct Board {
    logs: Vec<Log>
}
struct MultiBoard {
    boards: Vec<Board>,
}
pub struct LogFileHandler {
    multiboards: Vec<MultiBoard>,
}