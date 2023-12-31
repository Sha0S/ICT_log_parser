
const LANG_CHANGE: usize = 0;
//const INPUT_FOLDER: usize = 1;
const YIELD: usize = 2;
const MB_YIELD: usize = 3;
const FIRST_T: usize = 4;
const AFTER_RT: usize = 5;
const TOTAL: usize = 6;
const LOADING_MESSAGE: usize = 7;

const M_SIZE: usize = 8;
// HU - EN //
const MESSAGE:  [[&str;2];M_SIZE] = [
    ["Váltás magyar nyelvre!", "Language changed to English!"],
    ["Forrás:", "Source:"],
    ["Kihozatal:", "Yield:"],
    ["Multiboard:","As multiboards:"],
    ["Első teszt után:", "After first test:"],
    ["Végső kihozatal:","After retest:"],
    ["Összes teszt:", "All test:"],
    ["Logok betöltése", "Loadings logs"],
];