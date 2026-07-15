#!/usr/bin/env bash
# golden-test.sh — hydro-cli 数值 golden 回归。
#   用例布局: cli/golden/<calc>/case-*/ 下三件套
#     cmd            一行「<calc> <action>」
#     input.json     经 stdin 喂给 hydro-cli
#     expected.json  期望 stdout(canonical JSON 比较,键序无关)
#   改任何 calc 数学 → 本脚本必须全 PASS 才能 build/commit(advisory,build.sh 前手跑)。
set -uo pipefail
DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$DIR"   # golden input 里的相对路径(如 capacity case-3 的 xlsx)以仓根解析
BIN="$DIR/cli/target/release/hydro-cli"
[ -x "$BIN" ] || { echo "❌ 先 cargo build --release ($BIN 不存在)"; exit 1; }
TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT
PASS=0; FAIL=0
for case_dir in "$DIR"/cli/golden/*/case-*/; do
  [ -d "$case_dir" ] || continue
  name="$(echo "$case_dir" | sed "s|$DIR/cli/golden/||;s|/$||")"
  read -r calc action < "$case_dir/cmd"
  "$BIN" "$calc" "$action" < "$case_dir/input.json" > "$TMP" 2>/dev/null
  if python3 -c "
import json, sys
expected = json.load(open(sys.argv[1]))
actual = json.load(open(sys.argv[2]))
sys.exit(0 if expected == actual else 1)
" "$case_dir/expected.json" "$TMP"
  then echo "  ✅ $name ($calc $action)"; PASS=$((PASS+1))
  else echo "  🔴 $name ($calc $action) 输出 ≠ expected.json"; FAIL=$((FAIL+1)); fi
done
echo "golden: $PASS PASS / $FAIL FAIL"
[ "$FAIL" = 0 ]
