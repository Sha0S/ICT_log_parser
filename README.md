# Keysight ICT Logfile parser

![image](https://github.com/Sha0S/ICT_log_parser/assets/155308506/2628c041-becc-4539-938a-0a272e2d5a56)


A simple tool for reading and processing the text logfiles created by Keysights ICTs. (Specifically the i3700 series.)

It was made to replace one of my older Python scripts, which processed the logfiles into a spreadsheet.

Additional functionality:
- Yield report for individual boards and multiboards.
- Reports the failed tests in the logs.
- Reports hourly throughput.
- Has a small plot for viewing test results.

The export functionality has been expanded:
- Ability to select which tests to export. (All / Failed tests / Manually specified ones)
- Ability to only export logs which had failures.

# TODO:

- Verification.
- Localization.
- Export format improvements.
- Report more information about failures. (Position on the multiboard, DMC of the failed boards, etc.. )
