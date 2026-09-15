# HydroMac Water-Resources Toolbox

[中文](README.md) | **English**

A native macOS toolbox for water-resources calculations: **SwiftUI frontend + Rust calculation backend**, self-contained in a single .app and usable offline.

> A native macOS toolbox for water-resources engineering: 8 calculators (annual water-resources report query, river pollutant-carrying capacity, water-efficiency assessment, reservoir dispatch, river-district dispatch, irrigation demand, rainfall pipeline, geocoding) built as a SwiftUI shell driving a vendored Rust CLI over a JSON contract. Apple Silicon, macOS 14+. All sample data is synthetic.

## 8 calculators

| Calculator | Description |
|---|---|
| Annual water-resources report query | Query and compare annual water supply, water use, and socioeconomic indicators |
| Pollutant-carrying capacity | Capacity of river functional zones using a one-dimensional decay model, with Excel input support |
| Water-efficiency assessment | Three-level assessment of reclaimed-water use (large/small/point cycles, combined AHP + CRITIC weighting) |
| Reservoir-system dispatch | Joint dispatch simulation for two reservoirs (annual regulation, rule curves + special storage volumes) |
| River-district dispatch model | Water-balance dispatch across multiple zones in a plain river district (19-zone example) |
| Irrigation demand | Crop water demand in irrigation districts (single/double-cropping rice, dryland, and miscellaneous land; daily water balance) |
| Rainfall data analysis | Complete pipeline for zonal rainfall, water withdrawals, and Gaihu evaporation (6 steps, TSV input) |
| Geocoding | Batch address↔coordinate conversion (Amap API; key supplied through the `AMAP_KEY` environment variable) + WGS-84/GCJ-02 coordinate conversion |

## Architecture

```
Sources/*.swift   SwiftUI 前端(swiftc 直编,无 xcodeproj)
      │  Foundation.Process + JSON(stdin → stdout 信封 {ok,data,error})
      ▼
cli/  (cargo)     hydro-cli —— 纯 Rust 计算核(serde/calamine/csv/chrono)
```

- The backend binary ships inside the .app at `Contents/Resources/hydro-cli`, with no runtime dependencies.
- The calculation core vendors the pure Rust calc module from the author’s upstream hydro project. This repository includes 20 golden numerical regression cases (`scripts/golden-test.sh`).
- Example data and golden cases are **all synthetic demonstration data**, with fictional zone/reservoir/water-user names and no real engineering records.

## Installation

**From a Release**: Download `HydroMac-<版本>-arm64.zip`, extract it, and drag `水利工具箱.app` into Applications. The app is ad-hoc signed without a paid developer certificate. Clear the quarantine flag once before first launch:

```bash
xattr -cr "/Applications/水利工具箱.app"
```

Alternatively, right-click → Open, then allow it in System Settings → Privacy & Security.

**Build from source** (requires Xcode + the Rust toolchain):

```bash
git clone https://github.com/zengtianli/hydro-mac.git
cd hydro-mac
./build.sh --install
```

## Development

```bash
( cd cli && cargo build --release )      # 编后端
echo '' | cli/target/release/hydro-cli annual sample   # 单测某个计算器
bash scripts/golden-test.sh              # 20 case 数值回归(改任何 calc 数学必须全 PASS)
./build.sh                               # 完整构建 → build/水利工具箱.app
```

System requirements: macOS 14+, Apple Silicon.

## License

[MIT](LICENSE)

## Relationship to the private version

This repo is a **one-way snapshot** of the author's private build. It is refreshed
only when the private version cuts a release — day-to-day private commits are not
mirrored, so this repo may lag behind at any given time. (Policy set 2026-08-10.)
