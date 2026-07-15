//! 内嵌示例数据 —— 复刻自原 apps/district/src-tauri/src/sample_data.rs,
//! include_str! 路径改指本仓 cli/data/sample/district/。
use crate::district::types::*;

fn tsv(content: &str) -> TsvTable {
    let mut lines = content.trim().lines();
    let header_line = lines.next().unwrap_or("");
    let headers: Vec<String> = header_line.split('\t').map(|s| s.trim().to_string()).collect();
    let mut rows = Vec::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let cells: Vec<String> = line.split('\t').map(|s| s.trim().to_string()).collect();
        rows.push(cells);
    }
    TsvTable { headers, rows }
}

pub fn sample_input() -> SchedulerInput {
    let files = vec![
        ("HQ_ZQ".into(), tsv(include_str!("../../data/sample/district/static_HQ_ZQ.txt"))),
        ("HQ_SK".into(), tsv(include_str!("../../data/sample/district/static_HQ_SK.txt"))),
        ("SK".into(), tsv(include_str!("../../data/sample/district/input_SK.txt"))),
        ("SW_CS".into(), tsv(include_str!("../../data/sample/district/input_SW_CS.txt"))),
        ("SW_PS".into(), tsv(include_str!("../../data/sample/district/static_SW_PS.txt"))),
        ("SW_MB".into(), tsv(include_str!("../../data/sample/district/input_SW_MB.txt"))),
        ("FQJL".into(), tsv(include_str!("../../data/sample/district/input_FQJL.txt"))),
        ("LS_QT".into(), tsv(include_str!("../../data/sample/district/input_LS_QT.txt"))),
        ("XS_FN".into(), tsv(include_str!("../../data/sample/district/input_XS_FN.txt"))),
        ("XS_ST".into(), tsv(include_str!("../../data/sample/district/input_XS_ST.txt"))),
        ("GPS_GGXS".into(), tsv(include_str!("../../data/sample/district/input_GPS_GGXS.txt"))),
        ("GPS_PYCS".into(), tsv(include_str!("../../data/sample/district/input_GPS_PYCS.txt"))),
        ("FSSN_RULES".into(), tsv(include_str!("../../data/sample/district/static_FSSN_RULES.txt"))),
    ];
    SchedulerInput { files }
}
