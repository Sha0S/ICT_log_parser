#![allow(dead_code)]
#![allow(non_snake_case)]

/*
ToDo:
- Implement Export functions
*/

use std::ffi::OsString;
use std::ops::AddAssign;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::NaiveDateTime;
use umya_spreadsheet::{self, Worksheet};

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

fn get_next_free_name(tests: &Vec<Test>, base: String, counter: usize) -> (String,usize) {
    let name = format!("{}{}",base,counter);

    for t in tests {
        if t.name == name {
            return get_next_free_name(tests, base, counter+1);
        }
    }

    (name,counter+1)
}

// YYMMDDhhmmss => YY.MM.DD. hh:mm:ss
fn u64_to_string(mut x: u64) -> String {
    let YY = x/u64::pow(10, 10);
    x = x % u64::pow(10, 10);

    let MM = x/u64::pow(10, 8);
    x = x % u64::pow(10, 8);

    let DD = x/u64::pow(10, 6);
    x = x % u64::pow(10, 6);

    let hh = x/u64::pow(10, 4);
    x = x % u64::pow(10, 4);

    let mm = x/u64::pow(10, 2);
    x = x % u64::pow(10, 2);

    format!("{:02.0}.{:02.0}.{:02.0} {:02.0}:{:02.0}:{:02.0}", YY, MM, DD, hh, mm, x)
}

#[derive(PartialEq)]
pub enum ExportMode {
    All,
    FailuresOnly,
    Manual
}

pub struct ExportSettings {
    pub vertical: bool,
    pub only_failed_panels: bool,
    pub mode: ExportMode,
    pub list: String,
}

impl ExportSettings {
    pub fn default() -> Self {
        Self { vertical: false, only_failed_panels: false, mode: ExportMode::All, list: String::new() }
    }
}

pub type TResult = (BResult, f32);
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
pub enum TLimit {
    None,
    Lim2 (f32,f32),     // UL - LL
    Lim3 (f32,f32,f32)  // Nom - UL - LL
}

#[derive(Clone, Copy, PartialEq)]
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
            "RPT" => TType::Unknown, // Failure report, not a test
            "DPIN" => TType::Unknown,  // Failure report for testjet
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
pub enum BResult {
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

impl From<&str> for BResult {
    fn from(value: &str) -> Self {
        if matches!(value, "0" | "00") {
            return BResult::Pass
        }

        BResult::Fail
    }
}

impl BResult {
    pub fn print(&self) -> String {
        if matches!(self, BResult::Pass) {
            return String::from("Pass")
        }

        String::from("Fail")
    }
}

pub struct FailureList {
    pub test_id: usize,
    pub name: String,
    pub total: usize,
    //after_rt: usize,
    pub by_index: Vec<usize>
}

#[derive(Clone)]
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
    //pins_test: bool,

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
        //let mut pins_test: bool = true;
        let mut time_start: u64 = 0;
        let mut time_end: u64 = 0;
        let mut tests: Vec<Test> = Vec::new();

        // pre-populate pins test
        tests.push(
            Test { 
            name:   "pins".to_owned(),
            ttype:  TType::Pin,
            result: (BResult::Unknown,0.0),
            limits: TLimit::None }
        );
        //

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
                    // re-populate pins test
                    tests.push(
                        Test { 
                        name:   "pins".to_owned(),
                        ttype:  TType::Pin,
                        result: (BResult::Unknown,0.0),
                        limits: TLimit::None }
                    );
                    //
                }
                "{@PF" => {
                    tests[0].result.0 = parts.nth(2).unwrap().into();
                }
                "{@TS" => {
                    let tresult = parts.next().unwrap().into();
                                
                    let test = Test{    
                        name: strip_index(parts.last().unwrap()).to_string(),
                        ttype: TType::Shorts,
                        result: (tresult,0.0),
                        limits: TLimit::None};

                    tests.push(test);

                    _ = lines.next();
                }
                "{@TJET" => {
                    // WIP - testjet
                    // Are the testjet measuremenets always in a BLOCK? 
                    println!("ERR: Lone testjet test found! This is not implemented!!")
                }
                "{@D-T" => {
                    let tresult = parts.next().unwrap().into();
                                
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
                        result: (parts.next().unwrap().into(),0.0),
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

                    //if name.ends_with("testjet") { continue; }
                    
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
                            TType::Testjet => {
                                let tresult2 = parts.next().unwrap().into();
                                
                                let test = Test{    
                                    name: format!("{}%{}",name,strip_index(parts.last().unwrap())),
                                    ttype,
                                    result: (tresult2,0.0),
                                    limits: TLimit::None
                                };

                                tests.push(test);

                                _ = lines.next();
                            }
                            TType::Digital => {
                                let tresult2 = parts.next().unwrap().into();
                                let name2: String;

                                (name2, dt_counter) = get_next_free_name(&tests, format!("{}%digital_",strip_index(parts.last().unwrap())), dt_counter);
                                
                                let test = Test{    
                                    name: name2,
                                    ttype,
                                    result: (tresult2,0.0),
                                    limits: TLimit::None
                                };

                                tests.push(test);

                                _ = lines.next();
                            }
                            TType::BoundaryS => {
                                let name2: String;

                                (name2, bs_counter) = get_next_free_name(&tests, format!("{}%boundary_",strip_index(parts.next().unwrap())), bs_counter);

                                let test = Test {
                                    name: name2,
                                    ttype: TType::BoundaryS,
                                    result: (parts.next().unwrap().into(),0.0),
                                    limits: TLimit::None
                                };
            
                                tests.push(test);                                
                            }
                            _ => {
                                let mut name2 = name.clone();
                                let tresult2 = parts.next().unwrap().into();
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

    fn compare(&self, ordering: &Vec<TList>) -> bool {
        for (o,t) in ordering.iter().zip(self.tests.iter())
        {
            if o.0 != t.name {
                return false;
            }
        }

        true
    }

    fn order(&mut self, ordering: &Vec<TList>) -> bool {
        // Check if lengths match
        if ordering.len() < self.tests.len() {
            println!("\t\tTest list length mismatch!");
            return false
        }

        // Gather mismatching indexes
        let mut indexes: Vec<usize> = Vec::new();
        let mut list_tmp: Vec<Test> = Vec::new();
        for (i,(o,t)) in ordering.iter().zip(self.tests.iter()).enumerate()
        {
            if o.0 != t.name {
                println!("\t\t\t {} =/= {}", o.0, t.name);
                indexes.push(i);
                list_tmp.push(t.clone());
            }
        }

        println!("\t\tFound {} mismatching entries. {:?}" , indexes.len(), indexes);

        // Let's check if they even have a pair.
        'has_a_pair: for (i2, tmp) in list_tmp.iter_mut().enumerate() 
        {
            for (i, t) in ordering.iter().enumerate() {
                if tmp.name == t.0 {
                    // If it exists, but it isn't part of the indexes, then discard it.
                    // This *should* only affect "discharge" type tests. Which should be at the very end of the list.
                    if !indexes.contains(&i) {
                        println!("\t\tW: {}: {} has a pair (nbr {}) in the order, but it isn't present in the gathered indexes!", i2, tmp.name, i);
                        println!("\t\tW: Removing entry nbr {} from the testlist.", indexes[i2]);
                        self.tests.remove(indexes[i2]);
                    }

                    continue 'has_a_pair;
                }
            }

            println!("\t\tW: {}: {} has no pair in the order!", i2, tmp.name);
            
            // EXPERIMENTAL !!! 
            // This is a fix to a "bug" with Boundary Scan BLOCKS. 
            // If a block contains only BS measurements, in case of failure, it will only apear in the logfile as a single BS test.
            // This creates a incompatibility between the two. We can "fix" this by adding a index to the testname.
            let problematic_test = &mut self.tests[indexes[i2]];
            if problematic_test.ttype == TType::BoundaryS && problematic_test.result.0 == BResult::Fail{
                println!("\t\t\tIt's a failing BoundaryScan test, adding index to it's name.");
                problematic_test.name += "%boundary_1";
                tmp.name += "%boundary_1";
                println!("\t\t\tNew name: {}", problematic_test.name);
            }
            // EXPERIMENTAL !!!
        }

        // Copy the tmp list back to the testlist, with the indexes corrected.
        'fori: for i in &indexes
        {
            for tmp in list_tmp.iter()
            {
                if ordering[*i].0 == tmp.name {
                    self.tests[*i] = tmp.clone();
                    continue 'fori;
                }
            }

            println!("\t\tW: No match found for {}!!", ordering[*i].0);
        }

        println!("\t\tRe-ordering done!");

        self.compare(ordering) // could return "true" once I validated the fn
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

    fn export_to_col(&self, sheet: &mut Worksheet, mut c: u32) -> u32 {
        sheet.get_cell_mut((c, 1)).set_value(self.DMC.clone());
        sheet.get_cell_mut((c, 2)).set_value_number(self.index as u32);

        for l in &self.logs {
            sheet.get_cell_mut((c, 3)).set_value(l.result.print());
            sheet.get_cell_mut((c+1, 3)).set_value(u64_to_string(l.time_s));

            for (i,t) in l.results.iter().enumerate() {
                sheet.get_cell_mut((c, 4+(i as u32))).set_value(t.0.print());
                sheet.get_cell_mut((c+1, 4+(i as u32))).set_value_number(t.1);
            }

            c += 2; 
        }

        c
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

    fn get_failures(&self) -> Vec<(usize, usize)> {
        let mut failures: Vec<(usize, usize)> = Vec::new(); // (test number, board index)

        for b in &self.boards {
            for l in &b.logs {
                if l.result == BResult::Pass {
                    continue;
                }

                for (i, r) in l.results.iter().enumerate() {
                    if r.0 == BResult::Fail {
                        failures.push((i,b.index));
                    }
                }
            }
        }

        failures
    }

    // Get the measurments for test "testid". Vec<(time, index, result, limits)>
    fn get_stats_for_test(&self, testid: usize) -> Vec<(u64, usize, TResult, TLimit)> {
        let mut resultlist: Vec<(u64, usize, TResult, TLimit)> = Vec::new();

        for sb in &self.boards  {
            let index = sb.index;
            for l in &sb.logs {
                let time = l.time_s;
                if let Some(result) = l.results.get(testid) {
                    resultlist.push((
                        time,
                        index,
                        *result,
                        l.limits[testid]
                    ))
                }
            }
        }


        resultlist
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

    pub fn push(&mut self, mut log: LogFile) -> bool {
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
            if !log.compare(&self.testlist) {
                println!("\tW: Test order mismatch detected! Atempting re-ordering!\n\t\t{:?}", log.source);
                if !log.order(&self.testlist) {
                    println!("\t ERR: Re-ordering FAILED! Discrading the log.");
                    return false;
                }
                println!("\tINFO: Re orering succesfull!");
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

        self.pp_multiboard = 1;
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

    pub fn get_mb_yields(&self) -> [Yield; 3] {
        [self.mb_first_yield, self.mb_final_yield, self.mb_total_yield]
    }

    pub fn get_testlist(&self) -> &Vec<TList> {
        &self.testlist
    }

    pub fn get_failures(&self) -> Vec<FailureList> {
        let mut failure_list: Vec<FailureList> = Vec::new();

        for mb in &self.multiboards {
            'failfor: for failure in mb.get_failures() {
                // Check if already present
                for fl in &mut failure_list {
                    if fl.test_id == failure.0 {
                        fl.total += 1;
                        fl.by_index[failure.1-1] += 1;
                        continue 'failfor;
                    }
                }
                // If not make a new one
                let mut new_fail = FailureList {
                        test_id: failure.0,
                        name: self.testlist[failure.0].0.clone(),
                        total: 1, 
                        by_index: vec![0;self.pp_multiboard]};

                new_fail.by_index[failure.1-1] += 1;
                failure_list.push(new_fail );
                    
            }
        }

        failure_list.sort_by_key(|k| k.total);
        failure_list.reverse();

        /*for fail in &failure_list {
            println!("Test no {}, named {} failed {} times.", fail.test_id, fail.name, fail.total);
        } */

        failure_list
    }

    pub fn get_hourly_mb_stats(&self) -> Vec<(u64,usize,usize,Vec<(BResult,u64)>)> {
        // Vec<(time in yymmddhh, total ok, total nok, Vec<(result, mmss)> )>
        // Time is in format 231222154801 by default YYMMDDHHMMSS
        // We don't care about the last 4 digit, so we can div by 10^4

        let mut ret: Vec<(u64,usize,usize,Vec<(BResult,u64)>)> = Vec::new();

        for mb in &self.multiboards {
            'resfor: for res in &mb.results {
                let time = res.0 / u64::pow(10,4);
                let time_2 = res.0 % u64::pow(10,4);

                //println!("{} - {} - {}", res.0, time, time_2);

                // check if a entry for "time" exists
                for r in &mut ret {
                    if r.0 == time {
                        if res.1 == BResult::Pass {
                            r.1 += 1;
                        } else {
                            r.2 += 1; }
                        
                        r.3.push((res.1,time_2));

                        continue 'resfor;}
                } 

                ret.push((
                    time,
                    if res.1 == BResult::Pass {1} else {0},
                    if res.1 != BResult::Pass {1} else {0},
                    vec![(res.1,time_2)]
                ));
            }
        }

        ret.sort_by_key(|k| k.0);

        for r in &mut ret {
            r.3.sort_by_key(|k| k.1);
        }

        ret
    }

    // Get the measurments for test "testid". (TType,Vec<(time, index, result, limits)>) The Vec is sorted by time.
    // Could pass the DMC too
    pub fn get_stats_for_test(&self, testid: usize) -> (TType,Vec<(u64, usize, TResult, TLimit)>) {
        let mut resultlist: Vec<(u64, usize, TResult, TLimit)> = Vec::new();

        if testid > self.testlist.len() {
            println!("ERR: Test ID is out of bounds! {} > {}", testid, self.testlist.len());
            return (TType::Unknown,resultlist);
        }
        
        for mb in &self.multiboards  {
            resultlist.append(&mut mb.get_stats_for_test(testid));
        }

        resultlist.sort_by_key(|k| k.0);

        // let the time of resultlist[0] be t0, and each afterwards be tn-t0 in seconds.
        if let Some(&(t0,_,_,_)) = resultlist.first() {
            let nt0 = NaiveDateTime::parse_from_str(&format!("{t0}"),"%y%m%d%H%M%S").unwrap();

            for res in resultlist.iter_mut() {
                let ntn = NaiveDateTime::parse_from_str(&format!("{}",res.0),"%y%m%d%H%M%S").unwrap();
                //println!("{:?} - {:?}", ntn, nt0);
                res.0 = (ntn-nt0).num_seconds() as u64;
            }
        }

        (self.testlist[testid].1,resultlist)
    }

    fn get_first_ok_log(&self) -> Option<&Log> {
        for mb in &self.multiboards {
            for b in &mb.boards {
                for l in &b.logs {
                    if l.result == BResult::Pass {
                        return Some(l)
                    }
                }
            }
        }

        None
    }

    pub fn export(&self, path: PathBuf, settings: &ExportSettings) {
        let mut book = umya_spreadsheet::new_file();
        let mut sheet = book.get_sheet_mut(&0).unwrap();

        if settings.vertical {
            /* 
                WIP
            */
        }   else {
            sheet.get_cell_mut("A1").set_value(self.product_id.clone());
            sheet.get_cell_mut("A3").set_value("Test name");
            sheet.get_cell_mut("B3").set_value("Test type");
            sheet.get_cell_mut("D2").set_value("Test limits");
            sheet.get_cell_mut("C3").set_value("MIN");
            sheet.get_cell_mut("D3").set_value("Nom");
            sheet.get_cell_mut("E3").set_value("MAX");

            if settings.mode == ExportMode::All {
                for (i, t) in self.testlist.iter().enumerate() {
                    let l: u32 = (i + 4).try_into().unwrap(); 
                    sheet.get_cell_mut((1, l)).set_value(t.0.clone());
                    sheet.get_cell_mut((2, l)).set_value(t.1.print());
                }

                match self.get_first_ok_log() {
                    Some(l) => {
                        for (i,t) in l.limits.iter().enumerate() {
                            let l: u32 = (i + 4).try_into().unwrap();
                            // Lim2 (f32,f32),     // UL - LL
                            // Lim3 (f32,f32,f32)  // Nom - UL - LL
                            match t {
                                TLimit::Lim3(nom, ul, ll) => {
                                    sheet.get_cell_mut((3, l)).set_value_number(*ll);
                                    sheet.get_cell_mut((4, l)).set_value_number(*nom);
                                    sheet.get_cell_mut((5, l)).set_value_number(*ul);
                                }
                                TLimit::Lim2( ul, ll) => {
                                    sheet.get_cell_mut((3, l)).set_value_number(*ll);
                                    sheet.get_cell_mut((5, l)).set_value_number(*ul);
                                }
                                TLimit::None => {}
                            }
                        }
                    }
                    None => {
                        /* 
                            WIP
                        */                        
                        println!("ERR: No passing log found! WIP! Aborting!");
                    }
                };

                if settings.only_failed_panels {
                    /* 
                        WIP
                    */     
                } else {
                    let mut c: u32 = 6; 
                    for mb in &self.multiboards{
                        for b in &mb.boards {
                            c = b.export_to_col(sheet, c);
                        }
                    }
                }
            } else {
                /* 
                    WIP
                */
            }
        }

        let _ = umya_spreadsheet::writer::xlsx::write(&book, path);
    }
}