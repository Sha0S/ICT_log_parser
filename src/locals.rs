const LANG_HU: usize = 0;
const LANG_EN: usize = 1;

// SIDE PANEL
const LANG_CHANGE: usize = 0;
const LOADING_MESSAGE: usize = 1;
const SHIFT: usize = 2;
const A_DAY: usize = 3;
const LOAD: usize = 4;
const YIELD: usize = 5;
//const MB_YIELD: usize = 6;
const FIRST_T: usize = 7;
const AFTER_RT: usize = 8;
const TOTAL: usize = 9;
const FAILURES: usize = 10;
const PCS: usize = 11;
const AUTO_UPDATE: usize = 12;

const MESSAGE:  [[&str;2];13] = [
    ["Váltás magyar nyelvre!",  "Language changed to English!"],
    ["Logok betöltése",         "Loadings logs"],
    ["Műszak",                  "Shift"],
    ["24ó",                     "24h"],
    ["Betöltés",                "Load"],
    ["Kihozatal:",              "Yield:"],
    ["Multiboard:",             "As multiboards:"],
    ["Első teszt után:",        "After first test:"],
    ["Re-teszt után:",          "After retest:"],
    ["Összes teszt:",           "All tests:"],
    ["Kiesők",                  "Failures"],
    ["db",                      "pcs"],
    ["Automata frissítés:",     "Automatic update:"]
];

// EXPORT:

const EXPORT_LABEL: usize = 0;
const SETTINGS: usize = 1;
const VERTICAL_O: usize = 2;
const EXPORT_NOK_ONLY: usize = 3;
const EXPORT_MODE: usize = 4;
const EXPORT_MODE_ALL: usize = 5;
const EXPORT_MODE_FTO: usize = 6;
const EXPORT_MODE_MANUAL: usize = 7;
const EXPORT_MANUAL: usize = 8;
const EXPORT_MANUAL_EX: usize = 9;
const SAVE: usize = 10;

const MESSAGE_E: [[&str;2];11] = [
    ["💾 Export",                  "💾 Export"],
    ["Beállítások:",            "Settings:"],
    ["Vertikális elrendezés (1 sor = 1 log/pcb)",   "Vertical orientation (1 row = 1 log/pcb)"],
    ["Csak a kiesők logok exportálása",             "Export only the logs from failures"],
    ["Tesztek exportálása:",    "Export tests:"],
    ["Mindent",                 "All"],
    ["Csak a bukó teszteket",   "Only the failed tests"],
    ["Kézi tesztmegadás",       "Maunaly specify"],
    ["Kiválasztott tesztek:",    "Selected tests:"],
    ["Egy szóközzel válassza el a kívánt teszteket: Példa: \"c613 r412 v605%ON\"", 
                                "Separate tests with a space. Example: \"c613 r412 v605%ON\""],
    ["Mentés",                  "Save"]
];

// HOURLY + MULTIBOARDS:

const HOURLY_LABEL: usize = 0;
const TIME: usize = 1;
const RESULTS: usize = 2;
const MULTI_LABEL: usize = 3;

const MESSAGE_H: [[&str;2];4] = [
    ["⌚ Óránként",                "⌚ Hourly"],
    ["Időintervallum",          "Timeframe"],
    ["Eredmények",              "Results"],
    ["⌗ Multiboard-ok",           "⌗ Multiboards"],
];

// PLOT:

const PLOT_LABEL: usize = 0;

const MESSAGE_P: [[&str;2];1] = [
    ["📊 Grafikon",                "📊 Plotting"],
];