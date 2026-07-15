/// 河区名称 → 输出代码映射 (19 个河区)
pub const DISTRICT_NAME_MAPPING: &[(&str, &str)] = &[
    ("安和平原区", "output_hq_AHPYQ"),
    ("临溪平原上河区", "output_hq_LXPYSHQ"),
    ("临溪平原下河区", "output_hq_LXPYXHQ"),
    ("临溪平原澄江上游区", "output_hq_LXPYCJSYQ"),
    ("临溪平原澄江下游区", "output_hq_LXPYCJXYQ"),
    ("临溪平原柳浦中河区", "output_hq_LXPYLPZHQ"),
    ("青洲平原区", "output_hq_QZPYQ"),
    ("澄江沿线大工业用水区", "output_hq_CJYXDGYYSQ"),
    ("望潮平原东河区", "output_hq_WCPYDHQ"),
    ("望潮平原中河区", "output_hq_WCPYZHQ"),
    ("望潮平原西河区", "output_hq_WCPYXHQ"),
    ("岸洲滨湾平原区", "output_hq_AZBWPYQ"),
    ("晨湾平原区", "output_hq_CWPYQ"),
    ("泽原平原区", "output_hq_ZYPYQ"),
    ("环屿大陆用水区", "output_hq_HYDLYSQ"),
    ("泊北平原上河区", "output_hq_BBPYSHQ"),
    ("泊北平原中河区", "output_hq_BBPYZHQ"),
    ("黛山平原区", "output_hq_DSPYQ"),
    ("澜亭调水区", "output_hq_LTTSQ"),
];

/// 动态平衡河区 (其他外供 = 需水 - 河网供水 - 水库供水)
pub const BALANCED_DISTRICTS: &[&str] = &[
    "青洲平原区",
    "晨湾平原区",
    "泽原平原区",
    "黛山平原区",
];

/// 汇总列
pub const SUMMARY_COLUMNS: &[&str] = &[
    "平原产水", "其他外供", "河网供水", "水库供水", "合计来水",
    "农业需水", "其他生态需水", "非农需水", "需水量", "净流量",
    "目标容积", "日初容积", "日中容积", "日末容积", "排末容积", "水位生态需水",
    "初河蓄水", "蓄水消后", "缺水(浙东需供)", "末河蓄水", "排河蓄水",
    "排水容积", "河区排水", "容积变化", "排后变化", "纳蓄能力",
    "低水位以上蓄水量", "总蓄水量", "生态需水", "总需水量", "本地可供水量",
];

/// 输入文件 key → 文件名
pub const INPUT_FILES: &[(&str, &str)] = &[
    ("HQ_ZQ", "static_HQ_ZQ.txt"),
    ("HQ_SK", "static_HQ_SK.txt"),
    ("SK", "input_SK.txt"),
    ("SW_CS", "input_SW_CS.txt"),
    ("SW_PS", "static_SW_PS.txt"),
    ("SW_MB", "input_SW_MB.txt"),
    ("FQJL", "input_FQJL.txt"),
    ("LS_QT", "input_LS_QT.txt"),
    ("XS_FN", "input_XS_FN.txt"),
    ("XS_ST", "input_XS_ST.txt"),
    ("GPS_GGXS", "input_GPS_GGXS.txt"),
    ("GPS_PYCS", "input_GPS_PYCS.txt"),
    ("FSSN_RULES", "static_FSSN_RULES.txt"),
];

/// 文件分类
#[allow(dead_code)]
pub const DEMAND_FILES: &[&str] = &["XS_FN", "XS_ST", "GPS_GGXS"];
#[allow(dead_code)]
pub const INFLOW_FILES: &[&str] = &["FQJL", "LS_QT", "SK", "GPS_PYCS"];

/// 默认库容/水位列名
pub const VOLUME_COLUMNS: &[&str] = &["死库容", "低库容", "中库容", "高库容", "超蓄库容"];
pub const LEVEL_COLUMNS: &[&str] = &["死水位", "低水位", "中水位", "高水位", "超蓄水位"];

pub fn district_code(name: &str) -> &str {
    DISTRICT_NAME_MAPPING
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, c)| *c)
        .unwrap_or("output_hq_UNKNOWN")
}

pub fn is_balanced(name: &str) -> bool {
    BALANCED_DISTRICTS.contains(&name)
}
