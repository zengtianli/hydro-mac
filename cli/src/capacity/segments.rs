use super::physics::{capacity_value, outflow_concentration, velocity};
use super::types::{SegmentResult, Zone};
use std::collections::HashMap;

/// 分段计算功能区纳污能力 (含支流汇入)
///
/// Returns: (segments, total_W, final_C_out)
pub fn calc_zone_segments(
    zone: &Zone,
    main_q: f64,
    branch_flows: &HashMap<String, f64>,
) -> (Vec<SegmentResult>, f64, f64) {
    let mut segments = Vec::new();

    // 无支流或无流量 → 整段计算
    if zone.branches.is_empty() || main_q <= 0.0 {
        let u = velocity(main_q, zone.a, zone.beta);
        let w = capacity_value(zone.cs, zone.c0, main_q, u, zone.k, zone.length, zone.b);
        let c_out = outflow_concentration(zone.c0, zone.k, zone.length, u);

        segments.push(SegmentResult {
            name: if zone.main_name.is_empty() {
                zone.name.clone()
            } else {
                zone.main_name.clone()
            },
            seg_type: "干流段".into(),
            length: zone.length,
            q: main_q,
            c0: zone.c0,
            c_out,
            w,
            remark: "整段".into(),
        });

        segments.push(SegmentResult {
            name: format!("【{} 小计】", zone.name),
            seg_type: "汇总".into(),
            length: zone.length,
            q: 0.0,
            c0: zone.c0,
            c_out,
            w,
            remark: "仅汇总干流段".into(),
        });

        return (segments, w, c_out);
    }

    // 按汇入位置排序支流
    let mut sorted_branches = zone.branches.clone();
    sorted_branches.sort_by(|a, b| a.join_position.partial_cmp(&b.join_position).unwrap());

    let mut current_pos = 0.0_f64;
    let mut current_q = main_q;
    let mut current_c = zone.c0;
    let mut main_total_w = 0.0_f64;
    let mut total_length = 0.0_f64;
    let mut seg_num = 0u32;

    for (j, br) in sorted_branches.iter().enumerate() {
        let br_q = branch_flows.get(&br.name).copied().unwrap_or(0.0);

        // 干流段: current_pos → 汇入点
        let seg_length = br.join_position - current_pos;
        if seg_length > 0.0 && current_q > 0.0 {
            seg_num += 1;
            let u = velocity(current_q, zone.a, zone.beta);
            let seg_c_out = outflow_concentration(current_c, zone.k, seg_length, u);
            let seg_w = capacity_value(zone.cs, current_c, current_q, u, zone.k, seg_length, zone.b);

            let remark = if j == 0 {
                format!("起点→{}汇入点", br.name)
            } else {
                format!("上一汇入点→{}汇入点", br.name)
            };

            let seg_name = format!(
                "{}-段{}",
                if zone.main_name.is_empty() { &zone.name } else { &zone.main_name },
                seg_num
            );

            segments.push(SegmentResult {
                name: seg_name,
                seg_type: "干流段".into(),
                length: seg_length,
                q: current_q,
                c0: current_c,
                c_out: seg_c_out,
                w: seg_w,
                remark,
            });

            main_total_w += seg_w;
            total_length += seg_length;
            current_c = seg_c_out;
        }

        // 支流自身
        let br_c_out_final;
        if br_q > 0.0 && br.length > 0.0 {
            let br_u = velocity(br_q, zone.a, zone.beta);
            let br_c_out = outflow_concentration(br.c0, zone.k, br.length, br_u);
            let br_w = capacity_value(zone.cs, br.c0, br_q, br_u, zone.k, br.length, zone.b);

            segments.push(SegmentResult {
                name: br.name.clone(),
                seg_type: "支流".into(),
                length: br.length,
                q: br_q,
                c0: br.c0,
                c_out: br_c_out,
                w: br_w,
                remark: "(不计入汇总)".into(),
            });
            br_c_out_final = br_c_out;
        } else {
            br_c_out_final = br.c0;
        }

        // 混合点
        if br_q > 0.0 {
            let mixed_q = current_q + br_q;
            let mixed_c = if mixed_q > 0.0 {
                (current_q * current_c + br_q * br_c_out_final) / mixed_q
            } else {
                current_c
            };

            segments.push(SegmentResult {
                name: format!("(混合点{})", j + 1),
                seg_type: "混合".into(),
                length: 0.0,
                q: mixed_q,
                c0: mixed_c,
                c_out: 0.0,
                w: 0.0,
                remark: format!(
                    "Q={:.2}*C={:.4} + Q={:.2}*C={:.4}",
                    current_q, current_c, br_q, br_c_out_final
                ),
            });

            current_q = mixed_q;
            current_c = mixed_c;
        }

        current_pos = br.join_position;
    }

    // 最后一段干流 (最后汇入点 → 终点)
    let seg_length = zone.length - current_pos;
    let final_c_out;
    if seg_length > 0.0 && current_q > 0.0 {
        seg_num += 1;
        let u = velocity(current_q, zone.a, zone.beta);
        let seg_c_out = outflow_concentration(current_c, zone.k, seg_length, u);
        let seg_w = capacity_value(zone.cs, current_c, current_q, u, zone.k, seg_length, zone.b);

        let seg_name = format!(
            "{}-段{}",
            if zone.main_name.is_empty() { &zone.name } else { &zone.main_name },
            seg_num
        );

        segments.push(SegmentResult {
            name: seg_name,
            seg_type: "干流段".into(),
            length: seg_length,
            q: current_q,
            c0: current_c,
            c_out: seg_c_out,
            w: seg_w,
            remark: "最后汇入点→终点".into(),
        });

        main_total_w += seg_w;
        total_length += seg_length;
        final_c_out = seg_c_out;
    } else {
        final_c_out = current_c;
    }

    // 汇总行
    segments.push(SegmentResult {
        name: format!("【{} 小计】", zone.name),
        seg_type: "汇总".into(),
        length: total_length,
        q: 0.0,
        c0: zone.c0,
        c_out: final_c_out,
        w: main_total_w,
        remark: "仅汇总干流段".into(),
    });

    (segments, main_total_w, final_c_out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capacity::types::Branch;

    fn simple_zone() -> Zone {
        Zone {
            zone_id: "Z1".into(),
            name: "测试功能区".into(),
            water_class: "III".into(),
            length: 10000.0,
            k: 1e-6,
            b: 0.9,
            a: 0.05,
            beta: 0.6,
            cs: 0.2,
            c0: 0.1,
            main_name: "干流".into(),
            branches: vec![],
        }
    }

    #[test]
    fn test_no_branches() {
        let zone = simple_zone();
        let (segs, total_w, _c_out) = calc_zone_segments(&zone, 10.0, &HashMap::new());
        assert_eq!(segs.len(), 2); // 干流段 + 汇总
        assert!(total_w > 0.0);
        assert_eq!(segs[0].seg_type, "干流段");
        assert_eq!(segs[1].seg_type, "汇总");
    }

    #[test]
    fn test_with_branch() {
        let mut zone = simple_zone();
        zone.branches = vec![Branch {
            name: "支流1".into(),
            length: 2000.0,
            join_position: 5000.0,
            c0: 0.08,
        }];

        let mut branch_flows = HashMap::new();
        branch_flows.insert("支流1".into(), 3.0);

        let (segs, total_w, _c_out) = calc_zone_segments(&zone, 10.0, &branch_flows);

        // Should have: 干流段1, 支流, 混合点, 干流段2, 汇总
        assert_eq!(segs.len(), 5);
        assert!(total_w > 0.0);

        // Only 干流段 W counts in total
        let main_w_sum: f64 = segs
            .iter()
            .filter(|s| s.seg_type == "干流段")
            .map(|s| s.w)
            .sum();
        assert!((main_w_sum - total_w).abs() < 1e-10);
    }

    #[test]
    fn test_zero_flow() {
        let zone = simple_zone();
        let (_segs, total_w, c_out) = calc_zone_segments(&zone, 0.0, &HashMap::new());
        assert_eq!(total_w, 0.0);
        assert_eq!(c_out, zone.c0); // no decay with zero flow
    }
}
