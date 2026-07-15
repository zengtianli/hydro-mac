use crate::geocode::types::GeocodeResult;
use serde_json::Value;
use std::time::Duration;

const AMAP_REGEO_URL: &str = "https://restapi.amap.com/v3/geocode/regeo";
const AMAP_GEO_URL: &str = "https://restapi.amap.com/v3/geocode/geo";
const AMAP_POI_URL: &str = "https://restapi.amap.com/v3/place/text";

/// 安全提取字符串字段（跳过空列表等）
fn safe_str(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Array(_) => String::new(),
        Value::Null => String::new(),
        _ => val.to_string(),
    }
}

/// 逆地理编码: 经纬度 → 地址
pub async fn reverse_geocode(
    lng: f64,
    lat: f64,
    api_key: &str,
    index: usize,
) -> GeocodeResult {
    let client = reqwest::Client::new();
    let location = format!("{},{}", lng, lat);

    let resp = client
        .get(AMAP_REGEO_URL)
        .query(&[
            ("key", api_key),
            ("location", &location),
            ("extensions", "base"),
            ("output", "json"),
        ])
        .timeout(Duration::from_secs(10))
        .send()
        .await;

    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            return GeocodeResult {
                index,
                address: String::new(),
                province: String::new(),
                city: String::new(),
                district: String::new(),
                adcode: String::new(),
                lng: Some(lng),
                lat: Some(lat),
                error: Some(format!("network_error: {}", e)),
            };
        }
    };

    let data: Value = match resp.json().await {
        Ok(d) => d,
        Err(e) => {
            return GeocodeResult {
                index,
                address: String::new(),
                province: String::new(),
                city: String::new(),
                district: String::new(),
                adcode: String::new(),
                lng: Some(lng),
                lat: Some(lat),
                error: Some(format!("parse_error: {}", e)),
            };
        }
    };

    if data.get("status").and_then(|v| v.as_str()) != Some("1") {
        let info = data
            .get("info")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        return GeocodeResult {
            index,
            address: String::new(),
            province: String::new(),
            city: String::new(),
            district: String::new(),
            adcode: String::new(),
            lng: Some(lng),
            lat: Some(lat),
            error: Some(format!("amap_error: {}", info)),
        };
    }

    let regeocode = &data["regeocode"];
    let addr_comp = &regeocode["addressComponent"];

    GeocodeResult {
        index,
        address: safe_str(&regeocode["formatted_address"]),
        province: safe_str(&addr_comp["province"]),
        city: safe_str(&addr_comp["city"]),
        district: safe_str(&addr_comp["district"]),
        adcode: safe_str(&addr_comp["adcode"]),
        lng: Some(lng),
        lat: Some(lat),
        error: None,
    }
}

/// 正向地理编码: 地址 → 经纬度
pub async fn forward_geocode(
    address: &str,
    api_key: &str,
    city: Option<&str>,
    index: usize,
) -> GeocodeResult {
    let client = reqwest::Client::new();

    let mut params = vec![
        ("key", api_key),
        ("address", address),
        ("batch", "false"),
    ];
    let city_str;
    if let Some(c) = city {
        city_str = c.to_string();
        params.push(("city", &city_str));
    }

    let resp = client
        .get(AMAP_GEO_URL)
        .query(&params)
        .timeout(Duration::from_secs(10))
        .send()
        .await;

    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            return GeocodeResult {
                index,
                address: address.to_string(),
                province: String::new(),
                city: String::new(),
                district: String::new(),
                adcode: String::new(),
                lng: None,
                lat: None,
                error: Some(format!("network_error: {}", e)),
            };
        }
    };

    let data: Value = match resp.json().await {
        Ok(d) => d,
        Err(e) => {
            return GeocodeResult {
                index,
                address: address.to_string(),
                province: String::new(),
                city: String::new(),
                district: String::new(),
                adcode: String::new(),
                lng: None,
                lat: None,
                error: Some(format!("parse_error: {}", e)),
            };
        }
    };

    if data.get("status").and_then(|v| v.as_str()) != Some("1") {
        let info = data
            .get("info")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        return GeocodeResult {
            index,
            address: address.to_string(),
            province: String::new(),
            city: String::new(),
            district: String::new(),
            adcode: String::new(),
            lng: None,
            lat: None,
            error: Some(format!("amap_error: {}", info)),
        };
    }

    let geocodes = &data["geocodes"];
    if !geocodes.is_array() || geocodes.as_array().unwrap().is_empty() {
        return GeocodeResult {
            index,
            address: address.to_string(),
            province: String::new(),
            city: String::new(),
            district: String::new(),
            adcode: String::new(),
            lng: None,
            lat: None,
            error: Some("no_result".into()),
        };
    }

    let g = &geocodes[0];
    let location = g["location"].as_str().unwrap_or("");
    let (lng, lat) = parse_location(location);

    GeocodeResult {
        index,
        address: safe_str(&g["formatted_address"]),
        province: safe_str(&g["province"]),
        city: safe_str(&g["city"]),
        district: safe_str(&g["district"]),
        adcode: safe_str(&g["adcode"]),
        lng,
        lat,
        error: None,
    }
}

/// 企业名称搜索: 公司名 → POI 位置
pub async fn search_company(
    name: &str,
    api_key: &str,
    city: Option<&str>,
    index: usize,
) -> GeocodeResult {
    let client = reqwest::Client::new();

    let citylimit = if city.is_some() { "true" } else { "false" };
    let city_val = city.unwrap_or("");

    let resp = client
        .get(AMAP_POI_URL)
        .query(&[
            ("key", api_key),
            ("keywords", name),
            ("types", "120000|190000"),
            ("city", city_val),
            ("citylimit", citylimit),
            ("output", "json"),
        ])
        .timeout(Duration::from_secs(10))
        .send()
        .await;

    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            return GeocodeResult {
                index,
                address: String::new(),
                province: String::new(),
                city: String::new(),
                district: String::new(),
                adcode: String::new(),
                lng: None,
                lat: None,
                error: Some(format!("network_error: {}", e)),
            };
        }
    };

    let data: Value = match resp.json().await {
        Ok(d) => d,
        Err(e) => {
            return GeocodeResult {
                index,
                address: String::new(),
                province: String::new(),
                city: String::new(),
                district: String::new(),
                adcode: String::new(),
                lng: None,
                lat: None,
                error: Some(format!("parse_error: {}", e)),
            };
        }
    };

    if data.get("status").and_then(|v| v.as_str()) != Some("1") {
        let info = data
            .get("info")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        return GeocodeResult {
            index,
            address: String::new(),
            province: String::new(),
            city: String::new(),
            district: String::new(),
            adcode: String::new(),
            lng: None,
            lat: None,
            error: Some(format!("amap_error: {}", info)),
        };
    }

    let pois = &data["pois"];
    if !pois.is_array() || pois.as_array().unwrap().is_empty() {
        return GeocodeResult {
            index,
            address: String::new(),
            province: String::new(),
            city: String::new(),
            district: String::new(),
            adcode: String::new(),
            lng: None,
            lat: None,
            error: Some("no_result".into()),
        };
    }

    let poi = &pois[0];
    let location = poi["location"].as_str().unwrap_or("");
    let (lng, lat) = parse_location(location);

    // 拼接完整地址
    let province = safe_str(&poi["pname"]);
    let city_name = safe_str(&poi["cityname"]);
    let district = safe_str(&poi["adname"]);
    let detail = safe_str(&poi["address"]);

    let mut parts = Vec::new();
    if !province.is_empty() && province != "[]" {
        parts.push(province.clone());
    }
    if !city_name.is_empty() && city_name != "[]" && city_name != province {
        parts.push(city_name.clone());
    }
    if !district.is_empty() && district != "[]" && district != city_name {
        parts.push(district.clone());
    }
    if !detail.is_empty() && detail != "[]" {
        parts.push(detail);
    }
    let full_address = parts.join("");

    GeocodeResult {
        index,
        address: full_address,
        province,
        city: city_name,
        district,
        adcode: safe_str(&poi["adcode"]),
        lng,
        lat,
        error: None,
    }
}

/// 解析 "lng,lat" 字符串
fn parse_location(loc: &str) -> (Option<f64>, Option<f64>) {
    if let Some((lng_s, lat_s)) = loc.split_once(',') {
        let lng = lng_s.parse::<f64>().ok();
        let lat = lat_s.parse::<f64>().ok();
        (lng, lat)
    } else {
        (None, None)
    }
}
