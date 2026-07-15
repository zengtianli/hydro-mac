//! vendored from 上游 hydro 项目(私有)的 hydro-common —— 仅留 annual 用到的舍入。
//! 后续接 calamine 计算器(capacity/efficiency/reservoir/geocode)时,再补 cell_str/cell_f64。

/// 四舍五入到 2 位小数
pub fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// 四舍五入到 4 位小数
#[allow(dead_code)]
pub fn round4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}
