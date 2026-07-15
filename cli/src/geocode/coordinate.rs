use std::f64::consts::PI;

/// 判断是否在中国境外
fn out_of_china(lng: f64, lat: f64) -> bool {
    !(73.66 < lng && lng < 135.05 && 3.86 < lat && lat < 53.55)
}

/// 纬度转换辅助函数
fn transform_lat(lng: f64, lat: f64) -> f64 {
    let mut ret = -100.0 + 2.0 * lng + 3.0 * lat + 0.2 * lat * lat
        + 0.1 * lng * lat + 0.2 * lng.abs().sqrt();
    ret += (20.0 * (6.0 * lng * PI).sin() + 20.0 * (2.0 * lng * PI).sin()) * 2.0 / 3.0;
    ret += (20.0 * (lat * PI).sin() + 40.0 * (lat / 3.0 * PI).sin()) * 2.0 / 3.0;
    ret += (160.0 * (lat / 12.0 * PI).sin() + 320.0 * (lat * PI / 30.0).sin()) * 2.0 / 3.0;
    ret
}

/// 经度转换辅助函数
fn transform_lng(lng: f64, lat: f64) -> f64 {
    let mut ret = 300.0 + lng + 2.0 * lat + 0.1 * lng * lng
        + 0.1 * lng * lat + 0.1 * lng.abs().sqrt();
    ret += (20.0 * (6.0 * lng * PI).sin() + 20.0 * (2.0 * lng * PI).sin()) * 2.0 / 3.0;
    ret += (20.0 * (lng * PI).sin() + 40.0 * (lng / 3.0 * PI).sin()) * 2.0 / 3.0;
    ret += (150.0 * (lng / 12.0 * PI).sin() + 300.0 * (lng / 30.0 * PI).sin()) * 2.0 / 3.0;
    ret
}

/// WGS-84 坐标转 GCJ-02（火星坐标/高德坐标）
pub fn wgs84_to_gcj02(lng: f64, lat: f64) -> (f64, f64) {
    if out_of_china(lng, lat) {
        return (lng, lat);
    }

    let a: f64 = 6378245.0; // 长半轴
    let ee: f64 = 0.00669342162296594323; // 偏心率平方

    let dlat = transform_lat(lng - 105.0, lat - 35.0);
    let dlng = transform_lng(lng - 105.0, lat - 35.0);

    let radlat = lat / 180.0 * PI;
    let magic = 1.0 - ee * radlat.sin() * radlat.sin();
    let sqrtmagic = magic.sqrt();

    let dlat = (dlat * 180.0) / ((a * (1.0 - ee)) / (magic * sqrtmagic) * PI);
    let dlng = (dlng * 180.0) / (a / sqrtmagic * radlat.cos() * PI);

    let gcj_lng = lng + dlng;
    let gcj_lat = lat + dlat;

    (gcj_lng, gcj_lat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wgs84_to_gcj02_west_lake() {
        // 北京天安门 WGS-84
        let (gcj_lng, gcj_lat) = wgs84_to_gcj02(116.3976, 39.9086);
        // GCJ-02 should be slightly offset
        assert!((gcj_lng - 116.3976).abs() < 0.01);
        assert!((gcj_lat - 39.9086).abs() < 0.01);
        // Must not be identical (offset exists)
        assert!((gcj_lng - 116.3976).abs() > 0.001);
    }

    #[test]
    fn test_out_of_china_returns_unchanged() {
        let (lng, lat) = wgs84_to_gcj02(0.0, 51.5); // London
        assert_eq!(lng, 0.0);
        assert_eq!(lat, 51.5);
    }

    #[test]
    fn test_known_conversion() {
        // 天安门 WGS-84: 116.3912757, 39.906217
        let (gcj_lng, gcj_lat) = wgs84_to_gcj02(116.3912757, 39.906217);
        // GCJ-02 should be approximately 116.397, 39.908
        assert!((gcj_lng - 116.397).abs() < 0.005);
        assert!((gcj_lat - 39.908).abs() < 0.005);
    }
}
