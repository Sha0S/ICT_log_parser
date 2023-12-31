#![allow(dead_code)]
#![allow(non_snake_case)]

/*
ToDo:
- Multiboard level statistics?
- Implement Export functions
- Implement missing testtypes (pins/shorts/testjet)
*/

use std::ffi::OsString;
use std::ops::AddAssign;
use std::fs;
use std::path::Path;
//use umya_spreadsheet::*;

fn str_to_result(s: &str) -> bool {
    matches!(s, "0" | "00")
}

// Removes the index from the testname.
// For example: "17%c617" -> "c617"
fn strip_index(s: &str) -> &str {
    let mut chars = s.chars();

    let mut c = chars.next();
    while c.is_some() {
        if c.unwrap() == '%' {
            break;
        }
        c = chars.next();
    }

    chars.as_str()
}

type TResult = (bool, f32);
type TList  = (String, TType);

// OK - NOK
#[derive(Debug, Clone, Copy)]
pub struct Yield(pub u16,pub u16);
impl AddAssign for Yield {
    fn add_assign(&mut self, x: Self) {
       *self = Yield(self.0+x.0,self.1+x.1);
    }
}

// Returns Yield as a precentage (OK/(OK+NOK))*100
impl Yield {
    pub fn precentage(self) -> f32 {
        (self.0 as f32 *100.0)/( self.0 as f32 + self.1 as f32 )
    }
}

#[derive(Clone,Copy)]
enum TLimit {
    None,
    Lim2 (f32,f32),     // UL - LL
    Lim3 (f32,f32,f32)  // Nom - UL - LL
}

#[derive(Clone, Copy)]
pub enum TType {
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
    BoundaryS,
    Unknown //(String) // Do I need this?
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
            "BS-CON" => TType::BoundaryS,
            "RPT" => TType::Unknown,    
            _ => {
                println!("ERR: Unknown Test Type ! {}",s);
                TType::Unknown }
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
            TType::BoundaryS => "Boundary Scan".to_string(),
            TType::Unknown => "Unknown".to_string()
        }
    }
}


#[derive(Clone,Copy,PartialEq)]
enum BResult {
	Pass,
	Fail,
	Unknown 
}

impl From<BResult> for bool {
    fn from(val: BResult) -> Self {
        matches!(val, BResult::Pass)
    }
}

impl From<bool> for BResult {
    fn from(value: bool) -> Self {
        if value {
            return BResult::Pass
        }

        BResult::Fail
    }
}

struct Test {
    name: String,
    ttype: TType,

    result: TResult,
    limits: TLimit,
}

pub struct LogFile {
    source: OsString,
    DMC: String,
    DMC_mb: String,
    product_id: String,
    index: usize,

    result: bool,

    time_start: u64,
    time_end: u64,

    tests: Vec<Test>,
}

impl LogFile {
    pub fn load(p: &Path) -> Self {
        //println!("\tINFO: Loading file {}", p.display());
        let source = p.as_os_str().to_owned();
        let mut DMC = String::new();
        let mut DMC_mb = String::new();
        let mut product_id = String::new();
        let mut index: usize = 0;
        let mut result: bool = false;
        let mut time_start: u64 = 0;
        let mut time_end: u64 = 0;
        let mut tests: Vec<Test> = Vec::new();

        let fileb = fs::read_to_string(p).unwrap();
        let mut lines = fileb.lines();
        let mut iline = lines.next();

        //println!("\t\tINFO: Initailization done.");

        while iline.is_some() {
            let mut line = iline.unwrap().trim();
            if line == "}}" || line == "}" || line.is_empty() {
                iline = lines.next();
                continue;
            } 

            let mut parts = line.split('|');
            let word = parts.next();

            match word.unwrap() {
                "{@BATCH" => {
                    product_id = parts.next().unwrap().to_string();
                }
                "{@BTEST" => {
                    DMC = parts.next().unwrap().to_string();
                    result = str_to_result(parts.next().unwrap());
                    time_start = parts.next().unwrap().parse::<u64>().unwrap();
                    time_end = parts.nth(6).unwrap().parse::<u64>().unwrap();
                    index = parts.nth(1).unwrap().parse::<usize>().unwrap();
                    DMC_mb = parts.next().unwrap().to_string();

                    tests.clear(); // Someitmes BMW GW logs have capactiance compensation data at the start of the log. We don't want that.
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
                        name: strip_index(parts.last().unwrap()).to_string(),
                        ttype: TType::Digital,
                        result: (tresult,0.0),
                        limits: TLimit::None};

                    tests.push(test);

                    _ = lines.next();
                }
                "{@BS-CON" => {
                    let test = Test {
                        name: strip_index(parts.next().unwrap()).to_string(),
                        ttype: TType::BoundaryS,
                        result: (str_to_result(parts.next().unwrap()),0.0),
                        limits: TLimit::None
                    };

                    tests.push(test);
                }
                "{@RPT" => {
                    // report block -> ignored
                }
                "{@BLOCK" => {
                    let name = strip_index(parts.next().unwrap()).to_string();
                    let _tresult = str_to_result(parts.next().unwrap());
                    let mut dt_counter = 1;
                    let mut bs_counter = 1;

                    iline = lines.next();

                    if name.ends_with("testjet") { continue; }
                    
                    while iline.is_some() {
                        line = iline.unwrap().trim();
                        if line == "}" { break; } 

                        let mut part = line.split("{@");
                        parts = part.nth(1).unwrap().split('|');

                        let ttype = TType::new (parts.next().unwrap());

                        match ttype {
                            TType::Unknown => {
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
                                
                                let test = Test{    
                                    name: format!("{}:digital_{}",strip_index(parts.last().unwrap()), dt_counter),
                                    ttype,
                                    result: (tresult2,0.0),
                                    limits: TLimit::None
                                };

                                tests.push(test);
                                dt_counter += 1;

                                _ = lines.next();
                            }
                            TType::BoundaryS => {
                                let test = Test {
                                    name: format!("{}:boundary_{}",strip_index(parts.next().unwrap()),bs_counter),
                                    ttype: TType::BoundaryS,
                                    result: (str_to_result(parts.next().unwrap()),0.0),
                                    limits: TLimit::None
                                };
            
                                tests.push(test);
                                bs_counter += 1;
                            }
                            _ => {
                                let mut name2 = name.clone();
                                let tresult2 = str_to_result(parts.next().unwrap());
                                let measurement = parts.next().unwrap().parse::<f32>().unwrap();

                                if let Some(x) = parts.next() {
                                    name2 = name2 + "%" + x;
                                }

                                let limits:TLimit;
                                match part.next() {
                                    Some(x) => {
                                        parts = x.strip_suffix("}}").unwrap().split('|');
                                        match parts.next().unwrap() {
                                            "LIM2" => {
                                                limits = TLimit::Lim2 (
                                                    parts.next().unwrap().parse::<f32>().unwrap(),
                                                    parts.next().unwrap().parse::<f32>().unwrap());
                                            }
                                            "LIM3" => {
                                                limits = TLimit::Lim3 (
                                                    parts.next().unwrap().parse::<f32>().unwrap(),
                                                    parts.next().unwrap().parse::<f32>().unwrap(),
                                                    parts.next().unwrap().parse::<f32>().unwrap());
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
                                    name: name2,
                                    ttype,
                                    result: (tresult2, measurement),
                                    limits};

                                tests.push(test);
                            }
                        }

                        iline = lines.next();
                    }
                }
                _ => {
                    //println!("Unable to process {}", word.unwrap())
                }

            }

            iline = lines.next();            
        }

        //println!("\t\tINFO: Done.");

        Self {
            source,
            DMC,
            DMC_mb,
            product_id,
            index,
            result,
            time_start,
            time_end,
            tests
        }
    }
}

struct Log {
    time_s: u64,
    time_e: u64,
    result: BResult, // Could use a bool too, as it can't be Unknown

    results: Vec<TResult>,
    limits: Vec<TLimit>
}

impl Log {
    fn new(log: LogFile) -> Self {
        let mut results: Vec<TResult> = Vec::new();
        let mut limits: Vec<TLimit> = Vec::new();

        for t in log.tests {
            results.push(t.result);
            limits.push(t.limits);
        }
        
        Self {
            time_s: log.time_start,
            time_e: log.time_end,
            result: log.result.into(),
            results,
            limits
        }
    }
}

struct Board {
    DMC: String,
    logs: Vec<Log>,
    index: usize, // Number on the multiboard, goes from 1 to 20
}

impl Board {
    fn new(index: usize) -> Self {
        Self {
            DMC: String::new(),
            logs: Vec::new(),
            index,
        }
    }

    fn push(&mut self, log: LogFile) -> bool {
        // a) Board is empty
        if self.DMC.is_empty() {
            self.DMC = log.DMC.to_owned();
            self.logs.push(Log::new(log));
        // b) Board is NOT empty
        } else {
            self.logs.push(Log::new(log));
        }

        true
    }
}



struct MultiBoard {
    DMC: String,
    boards: Vec<Board>,

    // ( Start time, Multiboard test result, <Result of the individual boards>)
    results: Vec<(u64,BResult,Vec<BResult>)>,
}

impl MultiBoard {
    fn new() -> Self {
        Self {
            DMC: String::new(),
            boards: Vec::new(),
            results: Vec::new(),
            //first_res: BResult::Unknown,
            //final_res: BResult::Unknown
        }
    }

    // Q: should we check for the DMC of the board? If the main DMC and index is matching then it should be OK.
    fn push(&mut self, log: LogFile) -> bool {
        if self.DMC.is_empty() {
            self.DMC = log.DMC_mb.to_owned(); }

        while self.boards.len() < log.index {
            self.boards.push(Board::new(self.boards.len()+1))
        }

        self.boards[log.index-1].push(log)
    }

    // Generating stats for self, and reporting single-board stats.
    fn update(&mut self) -> (Yield,Yield,Yield) {
        let mut sb_first_yield  = Yield(0,0);
        let mut sb_final_yield  = Yield(0,0);
        let mut sb_total_yield  = Yield(0,0);

        /*for (i,b) in self.boards.iter().enumerate() {
            println!("{} has in pos {}, {} logs",self.DMC, i, b.logs.len());
            for (i2,l) in b.logs.iter().enumerate() {
                if l.result == BResult::Unknown {
                    println!("\t\t Log {} has result Unknown!", i2); }
            }
        }*/

        self.update_results();

        for (_,_,res) in &self.results {
            for r in res {
                if *r == BResult::Pass {
                    sb_total_yield.0 += 1;
                } else if *r == BResult::Fail {
                    sb_total_yield.1 += 1;
                }
            }
        }

        if let Some(x) = self.results.first() {
            for r in &x.2 {
                if *r == BResult::Pass {
                    sb_first_yield.0 += 1;
                } else if *r == BResult::Fail {
                    sb_first_yield.1 += 1;
                } else {
                    //println!("First is Unknown!");
                }
            }
        }

        if let Some(x) = self.results.last() {
            for r in &x.2 {
                if *r == BResult::Pass {
                    sb_final_yield.0 += 1;
                } else if *r == BResult::Fail {
                    sb_final_yield.1 += 1;
                } else {
                    //println!("Last is Unknown!");
                }
            }
        }

        (sb_first_yield,sb_final_yield,sb_total_yield)
    }

    fn update_results(&mut self) {
        for b in &self.boards {
            'forlog: for l in &b.logs {
                // 1 - check if there is a results with matching "time"
                for r in &mut self.results {
                    if r.0 == l.time_s {
                        // write the BResult in to r.2.index
                        r.2[b.index-1] = l.result;
                        continue 'forlog;
                    }
                }
                // 2 - if not then make one
                let mut new_res = (
                    l.time_s,
                    BResult::Unknown,
                    vec![BResult::Unknown;self.boards.len()]
                );
                new_res.2[b.index-1] = l.result;
                self.results.push(new_res);
            }
        }

        // At the end we have to update the 2nd filed of the results.
        for res in &mut self.results {
            let mut all_ok = true;
            let mut has_unknown = false;
            for r in &res.2 {
                match r {
                    BResult::Unknown => has_unknown = true,
                    BResult::Fail => all_ok = false,
                    _ => () 
                }
            }

            if !all_ok {
                res.1 = BResult::Fail;
            } else if has_unknown {
                res.1 = BResult::Unknown;
                //println!("MB marked as Unknown!");
            } else {
                res.1 = BResult::Pass
            }
        }

        // Sort results by time.
        self.results.sort_by_key(|k| k.0);
    }
}

pub struct LogFileHandler {
    // Statistics:
    pp_multiboard: usize,   // Panels Per Multiboard (1-20), can only be determined once everything is loaded. Might not need it.

    mb_first_yield: Yield,
    sb_first_yield: Yield,
    mb_final_yield: Yield,
    sb_final_yield: Yield,
    mb_total_yield: Yield,
    sb_total_yield: Yield,

    product_id: String,     // Product identifier
    testlist: Vec<TList>,
    multiboards: Vec<MultiBoard>
}

impl LogFileHandler {
    pub fn new() -> Self {
        LogFileHandler {
            pp_multiboard: 0,
            mb_first_yield: Yield(0,0),
            sb_first_yield: Yield(0,0),
            mb_final_yield: Yield(0,0),
            sb_final_yield: Yield(0,0),
            mb_total_yield: Yield(0,0),
            sb_total_yield: Yield(0,0),
            product_id: String::new(),
            testlist: Vec::new(),
            multiboards: Vec::new()
        }
    }

    pub fn push_from_file(&mut self, p: &Path) -> bool {
        //println!("INFO: Pushing file {} into log-stack", p.display());
        self.push(LogFile::load(p))
    }

    pub fn push(&mut self, log: LogFile) -> bool {
        if self.product_id.is_empty() {
            println!("\tINFO: Initializing as {}", log.product_id);
            self.product_id = log.product_id.to_owned();

            // Create testlist
            for t in log.tests.iter() {
                self.testlist.push((t.name.to_owned(), t.ttype));
            }

            self.multiboards.push(MultiBoard::new());
            self.multiboards[0].push(log)

        } else {
            // Check if it is for the same type.
            // Mismatched types are not supported. (And ATM I see no reason to do that.)
            if self.product_id != log.product_id {
                println!("\tERR: Product type mismatch detected! {} =/= {}\n\t\t {:?}", self.product_id, log.product_id, log.source);
                return false
            }

            // Check if the testlist matches
            // WIP
            //  I will have to latter add support for changes in the test order.
            //  Current itteration assumes the logs come from the same ICT, 
            //  and no major chages where made between tests!
            // WIP
            for (t1, t2) in self.testlist.iter().zip(log.tests.iter()) {
                if t1.0 != t2.name {
                    println!("\tERR: Test mismatch detected! {} =/= {} \n\t\t {:?}", t1.0, t2.name,log.source);
                    return false
                }
            }

            // If the new one has a longer testlist, then extend the current one.
            if self.testlist.len() < log.tests.len() {
                println!("\tINFO: Updating test_list.");
                self.testlist.clear();

                // I'm sure there is a faster way, but we rarely should have to do this, if ever.
                for t in &log.tests {
                    self.testlist.push((t.name.to_owned(), t.ttype));
                }
            }

			// Check if the MultiBoard already exists.
            for mb in self.multiboards.iter_mut() {
				if mb.DMC == log.DMC_mb {
					return mb.push(log)
				}
			}

            // If it does not, then make a new one
            let mut mb = MultiBoard::new();
            let rv = mb.push(log);
            self.multiboards.push(mb);
            rv
        }
    }

    pub fn update(&mut self) {
        println!("INFO: Update started...");
        let mut mbres: Vec<(Yield,Yield,Yield)> = Vec::new();

        self.pp_multiboard = 0;
        self.mb_first_yield  = Yield(0,0);
        self.mb_final_yield  = Yield(0,0);
        self.mb_total_yield  = Yield(0,0);

        for b in self.multiboards.iter_mut() {
            mbres.push(b.update());

            if self.pp_multiboard < b.boards.len() {
                self.pp_multiboard = b.boards.len();
            }

            for (_,r,_) in &b.results {
                if *r == BResult::Pass {
                    self.mb_total_yield.0 += 1;
                } else if *r == BResult::Fail {
                    self.mb_total_yield.1 += 1;
                }
            }

            if let Some(x) = b.results.first() {
                if x.1 == BResult::Pass {
                    self.mb_first_yield.0 += 1;
                } else if x.1 == BResult::Fail {
                    self.mb_first_yield.1 += 1;
                }
            }

            if let Some(x) = b.results.last() {
                if x.1 == BResult::Pass {
                    self.mb_final_yield.0 += 1;
                } else if x.1 == BResult::Fail {
                    self.mb_final_yield.1 += 1;
                }
            }
        }

        self.sb_first_yield  = Yield(0,0);
        self.sb_final_yield  = Yield(0,0);
        self.sb_total_yield  = Yield(0,0);

        for b in mbres {
            self.sb_first_yield += b.0;
            self.sb_final_yield += b.1;
            self.sb_total_yield += b.2;
        }       

        println!("INFO: Update done! Result: {:?} - {:?} - {:?}", self.sb_first_yield, self.sb_final_yield, self.sb_total_yield);
        println!("INFO: Update done! Result: {:?} - {:?} - {:?}", self.mb_first_yield, self.mb_final_yield, self.mb_total_yield);

    }

    pub fn clear(&mut self) {
        //self.pp_multiboard = 0;
        self.product_id = String::new();
        self.testlist = Vec::new();
        self.multiboards = Vec::new();
    }

    pub fn get_yields(&self) -> [Yield; 3] {
        [self.sb_first_yield, self.sb_final_yield, self.sb_total_yield]
    }

    pub fn get_testlist(&self) -> &Vec<TList> {
        &self.testlist
    }
}