#![allow(dead_code)]
#![allow(non_snake_case)]

use std::ffi::OsString;
use std::fs;
use std::ops::AddAssign;
use std::path::{Path, PathBuf};

use chrono::NaiveDateTime;
use umya_spreadsheet::{self, Worksheet};

mod keysight_log;

fn str_to_result(s: &str) -> bool {
    matches!(s, "0" | "00")
}

fn str_to_status(s: &str) -> &str {
    if let Ok(i) = s.parse::<i32>() {
        match i {
            0 => return "passed", 
            1 => return "uncategorized_failure",
            2 => return "failed_pin_test",
            3 => return "failed_in_learn_mode",
            4 => return "failed_shorts_test", 
            5 => return "(reserved)", 
            6 => return "failed_analog_test", 
            7 => return "failed_power_supply_test", 
            8 => return "failed_digital_or_boundary_scan_test", 
            9 => return "failed_functional_test", 
            10 => return "failed_pre-shorts_test", 
            11 => return "failed_in_board_handler", 
            12 => return "failed_barcode", 
            13 => return "X’d_out",
            14 => return "failed_in_VTEP_or_TestJet",
            15 => return "failed_in_polarity_check",
            16 => return "failed_in_ConnectCheck",
            17 => return "failed_in_analog_cluster_test",
            18..=79 => return "reserved",
            80 => return "runtime_error",
            81 => return "aborted_(STOP)",
            82 => return "aborted_(BREAK)",
            83..=89 => return "reserved",
            90..=99 => return "user-definable",
            _ => return "parsing_failed",
        }
    }

    ""
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

// For digital and boundaryscan tests.
// Base is "testname%digital_" or "testname%boundary_", and the function appends the next unused 'counter' to it.
fn get_next_free_name(tests: &Vec<Test>, base: String, counter: usize) -> (String, usize) {
    let name = format!("{}{}", base, counter);

    for t in tests {
        if t.name == name {
            return get_next_free_name(tests, base, counter + 1);
        }
    }

    (name, counter + 1)
}

// YYMMDDhhmmss => YY.MM.DD. hh:mm:ss
pub fn u64_to_string(mut x: u64) -> String {
    let YY = x / u64::pow(10, 10);
    x %= u64::pow(10, 10);

    let MM = x / u64::pow(10, 8);
    x %= u64::pow(10, 8);

    let DD = x / u64::pow(10, 6);
    x %= u64::pow(10, 6);

    let hh = x / u64::pow(10, 4);
    x %= u64::pow(10, 4);

    let mm = x / u64::pow(10, 2);
    x %= u64::pow(10, 2);

    format!(
        "{:02.0}.{:02.0}.{:02.0} {:02.0}:{:02.0}:{:02.0}",
        YY, MM, DD, hh, mm, x
    )
}

#[derive(Clone, Copy, PartialEq)]
pub enum ExportMode {
    All,
    FailuresOnly,
    Manual,
}

pub struct ExportSettings {
    pub vertical: bool,
    pub only_failed_panels: bool,
    pub only_final_logs: bool,
    pub mode: ExportMode,
    pub list: String,
}

impl ExportSettings {
    pub fn default() -> Self {
        Self {
            vertical: false,
            only_failed_panels: false,
            only_final_logs: false,
            mode: ExportMode::All,
            list: String::new(),
        }
    }
}

pub type TResult = (BResult, f32);
type TList = (String, TType);

// OK - NOK
#[derive(Debug, Clone, Copy)]
pub struct Yield(pub u16, pub u16);
impl AddAssign for Yield {
    fn add_assign(&mut self, x: Self) {
        *self = Yield(self.0 + x.0, self.1 + x.1);
    }
}

// Returns Yield as a precentage (OK/(OK+NOK))*100
impl Yield {
    pub fn precentage(self) -> f32 {
        (self.0 as f32 * 100.0) / (self.0 as f32 + self.1 as f32)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum TLimit {
    None,
    Lim2(f32, f32),      // UL - LL
    Lim3(f32, f32, f32), // Nom - UL - LL
}

#[derive(Clone, Copy, PartialEq)]
pub enum TType {
    Pin,
    Shorts,
    Jumper,
    Fuse,
    Resistor,
    Capacitor,
    Inductor,
    Diode,
    Zener,
    Testjet,
    Digital,
    Measurement,
    BoundaryS,
    Unknown,
}

impl TType {
    fn new(s: &str) -> Self {
        match s {
            "PF" => TType::Pin,
            "TS" => TType::Shorts,
            "A-JUM" => TType::Jumper,
            "A-FUS" => TType::Fuse,
            "A-RES" => TType::Resistor,
            "A-CAP" => TType::Capacitor,
            "A-DIO" => TType::Diode,
            "A-ZEN" => TType::Zener,
            "A-IND" => TType::Inductor,
            "TJET" => TType::Testjet,
            "D-T" => TType::Digital,
            "A-MEA" => TType::Measurement,
            "BS-CON" => TType::BoundaryS,
            "RPT" => TType::Unknown,  // Failure report, not a test
            "DPIN" => TType::Unknown, // Failure report for testjet
            _ => {
                println!("ERR: Unknown Test Type ! {}", s);
                TType::Unknown
            }
        }
    }

    fn print(&self) -> String {
        match self {
            TType::Pin => "Pin".to_string(),
            TType::Shorts => "Shorts".to_string(),
            TType::Jumper => "Jumper".to_string(),
            TType::Fuse => "Fuse".to_string(),
            TType::Resistor => "Resistor".to_string(),
            TType::Capacitor => "Capacitor".to_string(),
            TType::Inductor => "Inductor".to_string(),
            TType::Diode => "Diode".to_string(),
            TType::Zener => "Zener".to_string(),
            TType::Testjet => "Testjet".to_string(),
            TType::Digital => "Digital".to_string(),
            TType::Measurement => "Measurement".to_string(),
            TType::BoundaryS => "Boundary Scan".to_string(),
            TType::Unknown => "Unknown".to_string(),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum BResult {
    Pass,
    Fail,
    Unknown,
}

impl From<BResult> for bool {
    fn from(val: BResult) -> Self {
        matches!(val, BResult::Pass)
    }
}

impl From<bool> for BResult {
    fn from(value: bool) -> Self {
        if value {
            return BResult::Pass;
        }

        BResult::Fail
    }
}

impl From<&str> for BResult {
    fn from(value: &str) -> Self {
        if matches!(value, "0" | "00") {
            return BResult::Pass;
        }

        BResult::Fail
    }
}

impl BResult {
    pub fn print(&self) -> String {
        match self {
            BResult::Pass => String::from("Pass"),
            BResult::Fail => String::from("Fail"),
            BResult::Unknown => String::from("NA"),
        }
    }

    pub fn into_color(self) -> egui::Color32 {
        match self {
            BResult::Pass => egui::Color32::GREEN,
            BResult::Fail => egui::Color32::RED,
            BResult::Unknown => egui::Color32::YELLOW,
        }
    }

    pub fn into_dark_color(self) -> egui::Color32 {
        match self {
            BResult::Pass => egui::Color32::DARK_GREEN,
            BResult::Fail => egui::Color32::RED,
            BResult::Unknown => egui::Color32::BLACK,
        }
    }
}

pub struct FailureList {
    pub test_id: usize,
    pub name: String,
    pub total: usize,
    pub failed: Vec<(String, u64)>,
    pub by_index: Vec<usize>,
}

#[derive(Clone)]
struct Test {
    name: String,
    ttype: TType,

    result: TResult,
    limits: TLimit,
}

impl Test {
    fn clear(&mut self) {
        self.name = String::new();
        self.ttype = TType::Unknown;
        self.result = (BResult::Unknown, 0.0);
        self.limits = TLimit::None;
    }
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
    report: Vec<String>,
}

impl LogFile {
    pub fn load(p: &Path) -> Self {
        println!("INFO: Loading file {}", p.display());
        let source = p.as_os_str().to_owned();
        let mut DMC = String::new();
        let mut DMC_mb = String::new();
        let mut product_id = String::new();
        let mut index: usize = 0;
        let mut result: bool = false;
        let mut time_start: u64 = 0;
        let mut time_end: u64 = 0;
        let mut tests: Vec<Test> = Vec::new();
        let mut report: Vec<String> = Vec::new();

        let mut status_code = String::new();

        // pre-populate pins test
        tests.push(Test {
            name: "pins".to_owned(),
            ttype: TType::Pin,
            result: (BResult::Unknown, 0.0),
            limits: TLimit::None,
        });
        //

        let fileb = fs::read_to_string(p).unwrap();
        let mut lines = fileb.lines();
        let mut iline = lines.next();

        //println!("\t\tINFO: Initailization done.");

        while iline.is_some() {
            let mut line = iline.unwrap().trim().trim_end_matches('}');
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
                "{@BTEST" | "}{@BTEST" => {
                    DMC = parts.next().unwrap().to_string();

                    status_code = parts.next().unwrap().to_string();

                    result = str_to_result(&status_code);
                    time_start = parts.next().unwrap().parse::<u64>().unwrap();
                    time_end = parts.nth(6).unwrap().parse::<u64>().unwrap();
                    index = parts.nth(1).unwrap().parse::<usize>().unwrap();
                    DMC_mb = parts.next().unwrap().to_string();

                    tests.clear(); // Someitmes BMW GW logs have capactiance compensation data at the start of the log. We don't want that.
                                   // re-populate pins test
                    tests.push(Test {
                        name: "pins".to_owned(),
                        ttype: TType::Pin,
                        result: (BResult::Unknown, 0.0),
                        limits: TLimit::None,
                    });
                    //
                }
                "{@PF" => {
                    tests[0].result.0 = parts.nth(2).unwrap().into();
                }
                "{@TS" => {
                    let mut tresult = parts.next().unwrap().into();

                    // Sometimes, failed shorts tests are marked as passed at the 'test status' field.
                    // So we check the next 3 fields too, they all have to be '000'
                    for _ in 0..3 {
                        if parts.next().unwrap() != "000" {
                            tresult = BResult::Fail;
                        }
                    }

                    let test = Test {
                        name: strip_index(parts.last().unwrap()).to_string(),
                        ttype: TType::Shorts,
                        result: (tresult, 0.0),
                        limits: TLimit::None,
                    };

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

                    let test = Test {
                        name: strip_index(parts.last().unwrap()).to_string(),
                        ttype: TType::Digital,
                        result: (tresult, 0.0),
                        limits: TLimit::None,
                    };

                    tests.push(test);

                    //_ = lines.next();
                }
                "{@BS-CON" => {
                    let test = Test {
                        name: strip_index(parts.next().unwrap()).to_string(),
                        ttype: TType::BoundaryS,
                        result: (parts.next().unwrap().into(), 0.0),
                        limits: TLimit::None,
                    };

                    tests.push(test);
                }
                "{@RPT" => {
                    if let Some(rpt) = parts.next() {
                        report.push(rpt.trim_end_matches('}').to_owned());
                    }
                }
                "{@BLOCK" => {
                    let name = strip_index(parts.next().unwrap()).to_string();
                    let _tresult = str_to_result(parts.next().unwrap());
                    let mut dt_counter = 1;
                    let mut bs_counter = 1;

                    // PFT: Testjet and Digital measurements have a closing bracet too!
                    // We don't want to prematurely close the BLOCK, so we need to ignore these.
                    // Except these won't show up, if they fail, as the RPT block will folow them instead.
                    let mut potential_fake_terminator = false;

                    iline = lines.next();

                    while iline.is_some() {
                        line = iline.unwrap().trim();
                        if line == "}" {
                            if potential_fake_terminator {
                                iline = lines.next();
                                potential_fake_terminator = false;
                                continue;
                            } else {
                                break;
                            }
                        }
                        potential_fake_terminator = false;
                        line = line.trim_end_matches('}');

                        let mut part = line.split("{@");
                        parts = part.nth(1).unwrap().split('|');

                        let first = parts.next().unwrap();
                        if first == "RPT" {
                            if let Some(rpt) = parts.next() {
                                report.push(rpt.trim_end_matches('}').to_owned());
                            }

                            iline = lines.next();
                            continue;
                        }

                        let ttype = TType::new(first);

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

                                let test = Test {
                                    name: format!(
                                        "{}%{}",
                                        name,
                                        strip_index(parts.last().unwrap())
                                    ),
                                    ttype,
                                    result: (tresult2, 0.0),
                                    limits: TLimit::None,
                                };

                                tests.push(test);

                                potential_fake_terminator = true;
                            }
                            TType::Digital => {
                                let tresult2 = parts.next().unwrap().into();
                                let name2: String;

                                (name2, dt_counter) = get_next_free_name(
                                    &tests,
                                    format!("{}%digital_", strip_index(parts.last().unwrap())),
                                    dt_counter,
                                );

                                let test = Test {
                                    name: name2,
                                    ttype,
                                    result: (tresult2, 0.0),
                                    limits: TLimit::None,
                                };

                                tests.push(test);

                                potential_fake_terminator = true;
                            }
                            TType::BoundaryS => {
                                let name2: String;

                                (name2, bs_counter) = get_next_free_name(
                                    &tests,
                                    format!("{}%boundary_", strip_index(parts.next().unwrap())),
                                    bs_counter,
                                );

                                let test = Test {
                                    name: name2,
                                    ttype: TType::BoundaryS,
                                    result: (parts.next().unwrap().into(), 0.0),
                                    limits: TLimit::None,
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

                                let limits: TLimit;
                                match part.next() {
                                    Some(x) => {
                                        parts = x.trim_end_matches('}').split('|');
                                        match parts.next().unwrap() {
                                            "LIM2" => {
                                                limits = TLimit::Lim2(
                                                    parts.next().unwrap().parse::<f32>().unwrap(),
                                                    parts.next().unwrap().parse::<f32>().unwrap(),
                                                );
                                            }
                                            "LIM3" => {
                                                limits = TLimit::Lim3(
                                                    parts.next().unwrap().parse::<f32>().unwrap(),
                                                    parts.next().unwrap().parse::<f32>().unwrap(),
                                                    parts.next().unwrap().parse::<f32>().unwrap(),
                                                );
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

                                let test = Test {
                                    name: name2,
                                    ttype,
                                    result: (tresult2, measurement),
                                    limits,
                                };

                                tests.push(test);
                            }
                        }

                        iline = lines.next();
                    }
                }
                _ => {
                    println!("\tW: Unable to process {}", word.unwrap());
                }
            }

            iline = lines.next();
        }

        // Check for the case, when the status is set as failed, but we found no failing tests.
        if !result && !tests.iter().any(|f| f.result.0 == BResult::Fail ) {
            // Push in a dummy failed test
            tests.push(Test {
                name: format!("Status_code:{}_-_{}", status_code, str_to_status(&status_code)),
                ttype: TType::Unknown,
                result: (BResult::Fail, 0.0),
                limits: TLimit::None,
            });
        }

        Self {
            source,
            DMC,
            DMC_mb,
            product_id,
            index,
            result,
            time_start,
            time_end,
            tests,
            report,
        }
    }
}

struct Log {
    source: OsString,
    time_s: u64,
    time_e: u64,
    result: BResult, // Could use a bool too, as it can't be Unknown

    results: Vec<TResult>,
    limits: Vec<TLimit>,

    report: Vec<String>,
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
            source: log.source,
            time_s: log.time_start,
            time_e: log.time_end,
            result: log.result.into(),
            results,
            limits,
            report: log.report,
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

    fn update(&mut self) {
        self.logs.sort_by_key(|k| k.time_s);
    }

    fn all_ok(&self) -> bool {
        for l in &self.logs {
            if l.result == BResult::Fail {
                return false;
            }
        }
        true
    }

    fn get_reports(&self) -> Vec<String> {
        //let mut ret: Vec<String> = vec![format!("{} - {}", self.index, self.DMC)];
        let mut ret: Vec<String> = Vec::new();

        for (i, log) in self.logs.iter().enumerate() {
            if log.result == BResult::Pass {
                ret.push(format!("Log #{i} - {}: Pass\n", u64_to_string(log.time_e)));
            } else {
                ret.push(format!("Log #{i} - {}: Fail\n", u64_to_string(log.time_e)));

                if log.report.is_empty() {
                    ret.push(String::from("No report field found in log!\n"));
                    ret.push(String::from("Enable it in testplan!\n"));
                } else {
                    for rpt in &log.report {
                        ret.push(format!("\t{rpt}"));
                    }
                }
            }

            ret.push("\n".to_string());
        }

        ret
    }

    fn export_to_col(
        &self,
        sheet: &mut Worksheet,
        mut c: u32,
        only_failure: bool,
        only_final: bool,
        export_list: &[usize],
    ) -> u32 {
        if self.logs.is_empty() {
            return c;
        }

        if only_failure && self.all_ok() {
            return c;
        }

        if only_final && only_failure && self.logs.last().is_some_and(|x| x.result == BResult::Pass)
        {
            return c;
        }

        // Board values (DMC+index) only get printed once
        sheet.get_cell_mut((c, 1)).set_value(self.DMC.clone());
        sheet
            .get_cell_mut((c, 2))
            .set_value_number(self.index as u32);

        let log_slice = {
            if only_final {
                &self.logs[self.logs.len() - 1..]
            } else {
                &self.logs[..]
            }
        };

        for l in log_slice {
            if only_failure && l.result == BResult::Pass {
                continue;
            }

            // Log result and time of test
            sheet.get_cell_mut((c, 3)).set_value(l.result.print());
            sheet
                .get_cell_mut((c + 1, 3))
                .set_value(u64_to_string(l.time_s));

            // Print measurement results
            for (i, t) in export_list.iter().enumerate() {
                if let Some(res) = l.results.get(*t) {
                    if res.0 != BResult::Unknown {
                        sheet
                            .get_cell_mut((c, 4 + (i as u32)))
                            .set_value(res.0.print());
                        sheet
                            .get_cell_mut((c + 1, 4 + (i as u32)))
                            .set_value_number(res.1);
                    }
                }
            }
            c += 2;
        }

        c
    }

    fn export_to_line(
        &self,
        sheet: &mut Worksheet,
        mut l: u32,
        only_failure: bool,
        only_final: bool,
        export_list: &[usize],
    ) -> u32 {
        if self.logs.is_empty() {
            return l;
        }

        if only_failure && self.all_ok() {
            return l;
        }

        if only_final && only_failure && self.logs.last().is_some_and(|x| x.result == BResult::Pass)
        {
            return l;
        }

        // Board values (DMC+index) only get printed once
        sheet.get_cell_mut((1, l)).set_value(self.DMC.clone());

        let log_slice = {
            if only_final {
                &self.logs[self.logs.len() - 1..]
            } else {
                &self.logs[..]
            }
        };

        for log in log_slice {
            if only_failure && log.result == BResult::Pass {
                continue;
            }

            // Log result and time of test
            sheet.get_cell_mut((3, l)).set_value(log.result.print());
            sheet
                .get_cell_mut((2, l))
                .set_value(u64_to_string(log.time_s));

            // Print measurement results
            for (i, t) in export_list.iter().enumerate() {
                if let Some(res) = log.results.get(*t) {
                    if res.0 != BResult::Unknown {
                        let c = i as u32 * 2 + 4;
                        sheet.get_cell_mut((c, l)).set_value(res.0.print());
                        sheet.get_cell_mut((c + 1, l)).set_value_number(res.1);
                    }
                }
            }
            l += 1;
        }

        l
    }
}

#[derive(Clone)]
pub struct MbResult {
    pub start: u64,
    pub end: u64,
    pub result: BResult,
    pub panels: Vec<BResult>,
}
struct MultiBoard {
    DMC: String,
    boards: Vec<Board>,

    // ( Start time, Multiboard test result, <Result of the individual boards>)
    results: Vec<MbResult>,
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
            self.DMC = log.DMC_mb.to_owned();
        }

        while self.boards.len() < log.index {
            self.boards.push(Board::new(self.boards.len() + 1))
        }

        self.boards[log.index - 1].push(log)
    }

    // Generating stats for self, and reporting single-board stats.
    fn update(&mut self) -> (Yield, Yield, Yield) {
        let mut sb_first_yield = Yield(0, 0);
        let mut sb_final_yield = Yield(0, 0);
        let mut sb_total_yield = Yield(0, 0);

        for sb in &mut self.boards {
            sb.update();
        }

        self.update_results();

        for result in &self.results {
            for r in &result.panels {
                if *r == BResult::Pass {
                    sb_total_yield.0 += 1;
                } else if *r == BResult::Fail {
                    sb_total_yield.1 += 1;
                }
            }
        }

        if let Some(x) = self.results.first() {
            for r in &x.panels {
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
            for r in &x.panels {
                if *r == BResult::Pass {
                    sb_final_yield.0 += 1;
                } else if *r == BResult::Fail {
                    sb_final_yield.1 += 1;
                } else {
                    //println!("Last is Unknown!");
                }
            }
        }

        (sb_first_yield, sb_final_yield, sb_total_yield)
    }

    fn update_results(&mut self) {
        self.results.clear();

        for b in &self.boards {
            'forlog: for l in &b.logs {
                // 1 - check if there is a results with matching "time"
                for r in &mut self.results {
                    if r.start == l.time_s {
                        // write the BResult in to r.2.index
                        r.panels[b.index - 1] = l.result;

                        // if time_e is higher than the saved end time, then overwrite it
                        if r.end < l.time_e {
                            r.end = l.time_e;
                        }
                        continue 'forlog;
                    }
                }
                // 2 - if not then make one
                let mut new_res = MbResult {
                    start: l.time_s,
                    end: l.time_e,
                    result: BResult::Unknown,
                    panels: vec![BResult::Unknown; self.boards.len()],
                };
                new_res.panels[b.index - 1] = l.result;
                self.results.push(new_res);
            }
        }

        // At the end we have to update the 2nd field of the results.
        for res in &mut self.results {
            let mut all_ok = true;
            let mut has_unknown = false;
            for r in &res.panels {
                match r {
                    BResult::Unknown => has_unknown = true,
                    BResult::Fail => all_ok = false,
                    _ => (),
                }
            }

            if !all_ok {
                res.result = BResult::Fail;
            } else if has_unknown {
                res.result = BResult::Unknown;
            } else {
                res.result = BResult::Pass
            }
        }

        // Sort results by time.
        self.results.sort_by_key(|k| k.start);
    }

    fn get_results(&self) -> &Vec<MbResult> {
        &self.results
    }

    fn get_failures(&self, setting: FlSettings) -> Vec<(usize, usize, String, u64)> {
        let mut failures: Vec<(usize, usize, String, u64)> = Vec::new(); // (test number, board index, DMC, time)

        for b in &self.boards {
            if b.logs.is_empty() {
                continue;
            }

            let logs = match setting {
                FlSettings::All => &b.logs,
                FlSettings::AfterRetest => &b.logs[b.logs.len() - 1..],
                FlSettings::FirstPass => &b.logs[..1],
            };

            for l in logs {
                if l.result == BResult::Pass {
                    continue;
                }
                for (i, r) in l.results.iter().enumerate() {
                    if r.0 == BResult::Fail {
                        failures.push((i, b.index, b.DMC.clone(), l.time_s));
                    }
                }
            }
        }

        failures
    }

    // Get the measurments for test "testid". Vec<(time, index, result, limits)>
    fn get_stats_for_test(&self, testid: usize) -> Vec<(u64, usize, TResult, TLimit)> {
        let mut resultlist: Vec<(u64, usize, TResult, TLimit)> = Vec::new();

        for sb in &self.boards {
            let index = sb.index;
            for l in &sb.logs {
                let time = l.time_s;
                if let Some(result) = l.results.get(testid) {
                    resultlist.push((time, index, *result, l.limits[testid]))
                }
            }
        }

        resultlist
    }

    fn get_reports(&self) -> Vec<String> {
        let mut ret: Vec<String> = Vec::new();

        for sb in &self.boards {
            if !sb.all_ok() {
                ret.append(&mut sb.get_reports());
            }
        }

        ret
    }
}

pub struct LogFileHandler {
    // Statistics:
    pp_multiboard: usize, // Panels Per Multiboard (1-20), can only be determined once everything is loaded. Might not need it.

    mb_first_yield: Yield,
    sb_first_yield: Yield,
    mb_final_yield: Yield,
    sb_final_yield: Yield,
    mb_total_yield: Yield,
    sb_total_yield: Yield,

    product_id: String, // Product identifier
    testlist: Vec<TList>,
    multiboards: Vec<MultiBoard>,

    sourcelist: Vec<OsString>,
}

pub type HourlyStats = (u64, usize, usize, Vec<(BResult, u64, String)>); // (time, OK, NOK, Vec<Results>)
pub type MbStats = (String, Vec<MbResult>); // (DMC, Vec<(time, Multiboard result, Vec<Board results>)>)

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum FlSettings {
    FirstPass,
    All,
    AfterRetest,
}

impl LogFileHandler {
    pub fn new() -> Self {
        LogFileHandler {
            pp_multiboard: 0,
            mb_first_yield: Yield(0, 0),
            sb_first_yield: Yield(0, 0),
            mb_final_yield: Yield(0, 0),
            sb_final_yield: Yield(0, 0),
            mb_total_yield: Yield(0, 0),
            sb_total_yield: Yield(0, 0),
            product_id: String::new(),
            testlist: Vec::new(),
            multiboards: Vec::new(),
            sourcelist: Vec::new(),
        }
    }

    pub fn push_from_file(&mut self, p: &Path) -> bool {
        //println!("INFO: Pushing file {} into log-stack", p.display());
        self.push(LogFile::load(p))
    }

    pub fn push(&mut self, mut log: LogFile) -> bool {
        println!("\tProcessing logfile: {:?}", log.source);

        if self.sourcelist.iter().any(|f| f == &log.source) {
            println!("\t\tW: Logfile already loaded!");
            return false;
        }

        self.sourcelist.push(log.source.clone());

        if self.product_id.is_empty() {
            println!("\t\tINFO: Initializing as {}", log.product_id);
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
                println!(
                    "\t\tERR: Product type mismatch detected! {} =/= {}\n\t\t {:?}",
                    self.product_id, log.product_id, log.source
                );
                return false;
            }

            /*
                ToDo: Check for version (D5?)
                Need to add version info to logfile, and product_list.
            */

            // If the testlist is missing any entries, add them
            for test in &log.tests {
                if !self.testlist.iter().any(|e| e.0 == test.name) {
                    println!(
                        "\t\tW: Test {} was missing from testlist. Adding.",
                        test.name
                    );
                    self.testlist.push((test.name.clone(), test.ttype));
                }
            }

            // log.tests is always shorter or = than the testlist
            log.tests.resize(
                self.testlist.len(),
                Test {
                    name: String::new(),
                    ttype: TType::Unknown,
                    result: (BResult::Unknown, 0.0),
                    limits: TLimit::None,
                },
            );

            let len = log.tests.len(); // log.tests is always shorter than the testlist
            let mut buffer_i: Vec<usize> = Vec::new();

            // Get diff
            let mut q = 0;

            for i in 0..len {
                if self.testlist[i].0 != log.tests[i].name {
                    if !log.tests[i].name.is_empty() {
                        q += 1;
                        println!(
                            "\t\tW: Test mismatch: {} =/= {}",
                            self.testlist[i].0, log.tests[i].name
                        );
                    }
                    buffer_i.push(i);
                }
            }

            if q > 0 {
                print!(
                    "\t\tFound {} ({}) mismatches, re-ordering... ",
                    q,
                    buffer_i.len()
                );
                let mut tmp: Vec<Test> = Vec::new();
                for i in &buffer_i {
                    tmp.push(log.tests[*i].clone());
                    log.tests[*i].clear();
                }

                for i in &buffer_i {
                    for t in &tmp {
                        if self.testlist[*i].0 == t.name {
                            log.tests[*i] = t.clone();
                        }
                    }
                }

                println!("Done!");
            }

            // Check if the MultiBoard already exists.
            for mb in self.multiboards.iter_mut() {
                if mb.DMC == log.DMC_mb {
                    return mb.push(log);
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
        let mut mbres: Vec<(Yield, Yield, Yield)> = Vec::new();

        self.pp_multiboard = 1;
        self.mb_first_yield = Yield(0, 0);
        self.mb_final_yield = Yield(0, 0);
        self.mb_total_yield = Yield(0, 0);

        for b in self.multiboards.iter_mut() {
            mbres.push(b.update());

            if self.pp_multiboard < b.boards.len() {
                self.pp_multiboard = b.boards.len();
            }

            for result in &b.results {
                if result.result == BResult::Pass {
                    self.mb_total_yield.0 += 1;
                } else if result.result == BResult::Fail {
                    self.mb_total_yield.1 += 1;
                }
            }

            if let Some(x) = b.results.first() {
                if x.result == BResult::Pass {
                    self.mb_first_yield.0 += 1;
                } else if x.result == BResult::Fail {
                    self.mb_first_yield.1 += 1;
                }
            }

            if let Some(x) = b.results.last() {
                if x.result == BResult::Pass {
                    self.mb_final_yield.0 += 1;
                } else if x.result == BResult::Fail {
                    self.mb_final_yield.1 += 1;
                }
            }
        }

        self.sb_first_yield = Yield(0, 0);
        self.sb_final_yield = Yield(0, 0);
        self.sb_total_yield = Yield(0, 0);

        for b in mbres {
            self.sb_first_yield += b.0;
            self.sb_final_yield += b.1;
            self.sb_total_yield += b.2;
        }

        println!(
            "INFO: Update done! Result: {:?} - {:?} - {:?}",
            self.sb_first_yield, self.sb_final_yield, self.sb_total_yield
        );
        println!(
            "INFO: Update done! Result: {:?} - {:?} - {:?}",
            self.mb_first_yield, self.mb_final_yield, self.mb_total_yield
        );
    }

    pub fn clear(&mut self) {
        //self.pp_multiboard = 0;
        self.product_id = String::new();
        self.testlist = Vec::new();
        self.multiboards = Vec::new();
        self.sourcelist.clear();
    }

    pub fn get_yields(&self) -> [Yield; 3] {
        [
            self.sb_first_yield,
            self.sb_final_yield,
            self.sb_total_yield,
        ]
    }

    pub fn get_mb_yields(&self) -> [Yield; 3] {
        [
            self.mb_first_yield,
            self.mb_final_yield,
            self.mb_total_yield,
        ]
    }

    pub fn get_testlist(&self) -> &Vec<TList> {
        &self.testlist
    }

    pub fn get_failures(&self, setting: FlSettings) -> Vec<FailureList> {
        let mut failure_list: Vec<FailureList> = Vec::new();

        for mb in &self.multiboards {
            'failfor: for failure in mb.get_failures(setting) {
                // Check if already present
                for fl in &mut failure_list {
                    if fl.test_id == failure.0 {
                        fl.total += 1;
                        fl.failed.push((failure.2, failure.3));
                        fl.by_index[failure.1 - 1] += 1;
                        continue 'failfor;
                    }
                }
                // If not make a new one
                let mut new_fail = FailureList {
                    test_id: failure.0,
                    name: self.testlist[failure.0].0.clone(),
                    total: 1,
                    failed: vec![(failure.2, failure.3)],
                    by_index: vec![0; self.pp_multiboard],
                };

                new_fail.by_index[failure.1 - 1] += 1;
                failure_list.push(new_fail);
            }
        }

        failure_list.sort_by_key(|k| k.total);
        failure_list.reverse();

        /*for fail in &failure_list {
            println!("Test no {}, named {} failed {} times.", fail.test_id, fail.name, fail.total);
        } */

        failure_list
    }

    pub fn get_hourly_mb_stats(&self) -> Vec<HourlyStats> {
        // Vec<(time in yymmddhh, total ok, total nok, Vec<(result, mmss)> )>
        // Time is in format 231222154801 by default YYMMDDHHMMSS
        // We don't care about the last 4 digit, so we can div by 10^4

        let mut ret: Vec<HourlyStats> = Vec::new();

        for mb in &self.multiboards {
            'resfor: for res in &mb.results {
                let time = res.end / u64::pow(10, 4);
                let time_2 = res.end % u64::pow(10, 4);

                //println!("{} - {} - {}", res.0, time, time_2);

                // check if a entry for "time" exists
                for r in &mut ret {
                    if r.0 == time {
                        if res.result == BResult::Pass {
                            r.1 += 1;
                        } else {
                            r.2 += 1;
                        }

                        r.3.push((res.result, time_2, mb.DMC.clone()));

                        continue 'resfor;
                    }
                }

                ret.push((
                    time,
                    if res.result == BResult::Pass { 1 } else { 0 },
                    if res.result != BResult::Pass { 1 } else { 0 },
                    vec![(res.result, time_2, mb.DMC.clone())],
                ));
            }
        }

        ret.sort_by_key(|k| k.0);

        for r in &mut ret {
            r.3.sort_by_key(|k| k.1);
        }

        ret
    }

    // Returns the result of eaxh mb. Format: (DMC, Vec<(test_time, mb_result, Vec<board_result>)>)
    pub fn get_mb_results(&self) -> Vec<MbStats> {
        let mut ret: Vec<MbStats> = Vec::new();

        for mb in &self.multiboards {
            ret.push((mb.DMC.clone(), mb.get_results().clone()));
        }

        ret.sort_by_key(|k| k.1.last().unwrap().start);
        ret
    }

    // Get the measurments for test "testid". (TType,Vec<(time, index, result, limits)>) The Vec is sorted by time.
    // Could pass the DMC too
    pub fn get_stats_for_test(&self, testid: usize) -> (TType, Vec<(u64, usize, TResult, TLimit)>) {
        let mut resultlist: Vec<(u64, usize, TResult, TLimit)> = Vec::new();

        if testid > self.testlist.len() {
            println!(
                "ERR: Test ID is out of bounds! {} > {}",
                testid,
                self.testlist.len()
            );
            return (TType::Unknown, resultlist);
        }

        for mb in &self.multiboards {
            resultlist.append(&mut mb.get_stats_for_test(testid));
        }

        resultlist.sort_by_key(|k| k.0);

        for res in resultlist.iter_mut() {
            res.0 = NaiveDateTime::parse_from_str(&format!("{}", res.0), "%y%m%d%H%M%S")
                .unwrap()
                .and_utc()
                .timestamp() as u64;
        }

        (self.testlist[testid].1, resultlist)
    }

    pub fn get_tests_w_limit_changes(&self) -> Option<Vec<(usize, String)>> {
        let mut ret: Vec<(usize, String)> = Vec::new();

        'outerloop: for (i, (tname, ttype)) in self.testlist.iter().enumerate() {
            match ttype {
                // These tests have no "limit" by default, skip them
                TType::BoundaryS => continue,
                TType::Digital => continue,
                TType::Pin => continue,
                TType::Shorts => continue,
                TType::Testjet => continue,
                TType::Unknown => {
                    // this shouldn't happen
                    println!("ERR: TType::Unknown in the final testlist at #{i}, name {tname}");
                }
                _ => {
                    let mut limit: Option<&TLimit> = None;
                    for mb in &self.multiboards {
                        for sb in &mb.boards {
                            for log in &sb.logs {
                                if let Some(test) = log.limits.get(i) {
                                    if *test == TLimit::None {
                                        continue;
                                    }
                                    if limit.is_none() {
                                        limit = Some(test)
                                    } else if *limit.unwrap() != *test {
                                        println!(
                                            "INFO: Test {tname} has limit changes in the sample"
                                        );
                                        ret.push((i, tname.clone()));
                                        continue 'outerloop;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if ret.is_empty() {
            None
        } else {
            Some(ret)
        }
    }
    // We don't check here if the limits have changed.
    // Can't properly show than in a spreadsheet anyway.
    // We will notify the user about it in the UI only.
    fn generate_limit_list(&self) -> Option<Vec<TLimit>> {
        let mut ret: Vec<TLimit> = Vec::new();

        'outerloop: for i in 0..self.testlist.len() {
            for mb in &self.multiboards {
                for sb in &mb.boards {
                    for log in &sb.logs {
                        if let Some(limit) = log.limits.get(i) {
                            if log.results[i].0 != BResult::Unknown {
                                ret.push(*limit);
                                continue 'outerloop;
                            }
                        }
                    }
                }
            }
        }

        if ret.is_empty() {
            None
        } else {
            Some(ret)
        }
    }

    fn get_export_list(&self, settings: &ExportSettings) -> Vec<usize> {
        let mut ret: Vec<usize> = Vec::new();

        match settings.mode {
            ExportMode::All => {
                ret = (0..self.testlist.len()).collect();
            }
            ExportMode::FailuresOnly => {
                for id in self.get_failures(FlSettings::All) {
                    ret.push(id.test_id);
                }
            }
            ExportMode::Manual => {
                for part in settings.list.split(' ') {
                    for (i, (t, _)) in self.testlist.iter().enumerate() {
                        if *t == part {
                            ret.push(i);
                            break;
                        }
                    }
                }
            }
        }

        ret
    }

    pub fn export(&self, path: PathBuf, settings: &ExportSettings) {
        let mut book = umya_spreadsheet::new_file();
        let sheet = book.get_sheet_mut(&0).unwrap();

        if settings.vertical {
            // Create header
            sheet.get_cell_mut("A1").set_value(self.product_id.clone());
            sheet.get_cell_mut("A3").set_value("DMC");
            sheet.get_cell_mut("B3").set_value("Test time");
            sheet.get_cell_mut("C3").set_value("Log result");
            sheet.get_cell_mut("C1").set_value("Test name:");
            sheet.get_cell_mut("C2").set_value("Test limits:");

            // Generate list of teststeps to be exported
            let export_list = self.get_export_list(settings);

            // Print testlist
            for (i, t) in export_list.iter().enumerate() {
                let c: u32 = (i * 2 + 4).try_into().unwrap();
                sheet
                    .get_cell_mut((c, 1))
                    .set_value(self.testlist[*t].0.clone());
                sheet
                    .get_cell_mut((c + 1, 1))
                    .set_value(self.testlist[*t].1.print());

                sheet.get_cell_mut((c, 3)).set_value("Result");
                sheet.get_cell_mut((c + 1, 3)).set_value("Value");
            }

            // Print limits. Nominal value is skiped.
            // It does not check if the limit changed.
            if let Some(limits) = self.generate_limit_list() {
                for (i, t) in export_list.iter().enumerate() {
                    let c: u32 = (i * 2 + 4).try_into().unwrap();
                    // Lim2 (f32,f32),     // UL - LL
                    // Lim3 (f32,f32,f32)  // Nom - UL - LL
                    match limits[*t] {
                        TLimit::Lim3(_, ul, ll) => {
                            sheet.get_cell_mut((c, 2)).set_value_number(ll);
                            sheet.get_cell_mut((c + 1, 2)).set_value_number(ul);
                        }
                        TLimit::Lim2(ul, ll) => {
                            sheet.get_cell_mut((c, 2)).set_value_number(ll);
                            sheet.get_cell_mut((c + 1, 2)).set_value_number(ul);
                        }
                        TLimit::None => {}
                    }
                }
            }

            // Print test results
            let mut l: u32 = 4;
            for mb in &self.multiboards {
                for b in &mb.boards {
                    l = b.export_to_line(
                        sheet,
                        l,
                        settings.only_failed_panels,
                        settings.only_final_logs,
                        &export_list,
                    );
                }
            }
        } else {
            // Create header
            sheet.get_cell_mut("A1").set_value(self.product_id.clone());
            sheet.get_cell_mut("A3").set_value("Test name");
            sheet.get_cell_mut("B3").set_value("Test type");
            sheet.get_cell_mut("D2").set_value("Test limits");
            sheet.get_cell_mut("C3").set_value("MIN");
            sheet.get_cell_mut("D3").set_value("Nom");
            sheet.get_cell_mut("E3").set_value("MAX");

            // Generate list of teststeps to be exported
            let export_list = self.get_export_list(settings);

            // Print testlist
            for (i, t) in export_list.iter().enumerate() {
                let l: u32 = (i + 4).try_into().unwrap();
                sheet
                    .get_cell_mut((1, l))
                    .set_value(self.testlist[*t].0.clone());
                sheet
                    .get_cell_mut((2, l))
                    .set_value(self.testlist[*t].1.print());
            }

            // Print limits
            // It does not check if the limit changed.
            if let Some(limits) = self.generate_limit_list() {
                for (i, t) in export_list.iter().enumerate() {
                    let l: u32 = (i + 4).try_into().unwrap();
                    // Lim2 (f32,f32),     // UL - LL
                    // Lim3 (f32,f32,f32)  // Nom - UL - LL
                    match limits[*t] {
                        TLimit::Lim3(nom, ul, ll) => {
                            sheet.get_cell_mut((3, l)).set_value_number(ll);
                            sheet.get_cell_mut((4, l)).set_value_number(nom);
                            sheet.get_cell_mut((5, l)).set_value_number(ul);
                        }
                        TLimit::Lim2(ul, ll) => {
                            sheet.get_cell_mut((3, l)).set_value_number(ll);
                            sheet.get_cell_mut((5, l)).set_value_number(ul);
                        }
                        TLimit::None => {}
                    }
                }
            }

            // Print test results
            let mut c: u32 = 6;
            for mb in &self.multiboards {
                for b in &mb.boards {
                    c = b.export_to_col(
                        sheet,
                        c,
                        settings.only_failed_panels,
                        settings.only_final_logs,
                        &export_list,
                    );
                }
            }
        }

        let _ = umya_spreadsheet::writer::xlsx::write(&book, path);
    }

    fn get_mb_w_DMC(&self, DMC: &str) -> Option<&MultiBoard> {
        for mb in self.multiboards.iter() {
            for sb in &mb.boards {
                if sb.DMC == DMC {
                    return Some(mb);
                }
            }
        }

        println!("Found none as {DMC}");
        None
    }

    fn get_sb_w_DMC(&self, DMC: &str) -> Option<&Board> {
        for mb in self.multiboards.iter() {
            for sb in &mb.boards {
                if sb.DMC == DMC {
                    return Some(sb);
                }
            }
        }

        println!("Found none as {DMC}");
        None
    }

    pub fn get_report_for_MB(&self, DMC: &str) -> Option<Vec<String>> {
        if let Some(board) = self.get_mb_w_DMC(DMC) {
            return Some(board.get_reports());
        }

        None
    }

    pub fn get_report_for_SB(&self, DMC: &str) -> Option<Vec<String>> {
        if let Some(board) = self.get_sb_w_DMC(DMC) {
            return Some(board.get_reports());
        }

        None
    }

    pub fn get_report_for_SB_w_index(&self, DMC: &str, index: usize) -> Option<Vec<String>> {
        if let Some(mb) = self.get_mb_w_DMC(DMC) {
            if let Some(board) = mb.boards.get(index) {
                return Some(board.get_reports());
            }
        }

        None
    }

    pub fn get_report_for_SB_NOK(&self, DMC: &str) -> Option<Vec<String>> {
        if let Some(mb) = self.get_mb_w_DMC(DMC) {
            for sb in mb.boards.iter() {
                if !sb.all_ok() {
                    return Some(sb.get_reports());
                }
            }
        }

        None
    }
}
