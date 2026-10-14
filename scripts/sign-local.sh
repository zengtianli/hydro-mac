#!/bin/bash
# Explicit local iteration signing. No notarization, installation or release source.
set -euo pipefail
[ "$#" = 2 ] || { echo 'usage: sign-local.sh INPUT.app OUTPUT_DIRECTORY' >&2; exit 2; }
INPUT="$1"
STAGE="$2"
IDENTITY="${CODE_SIGN_IDENTITY:?Specify the stable Developer ID Application identity}"
[[ "$IDENTITY" == 'Developer ID Application: '* ]] || { echo 'Stable Developer ID required' >&2; exit 2; }
[ ! -e "$STAGE" ] || { echo 'Signing output exists; read the original result before resuming' >&2; exit 1; }
[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$INPUT/Contents/Info.plist")" = io.github.zengtianli.HydroMac ] || exit 2
security find-identity -v -p codesigning | /usr/bin/grep -qF "\"$IDENTITY\""
mkdir -p "$STAGE"
APP="$STAGE/水利工具箱.app"
ditto "$INPUT" "$APP"
python3 - "$INPUT" "$STAGE" "$IDENTITY" <<'PY'
import hashlib,json,sys
from pathlib import Path
source,stage,identity=Path(sys.argv[1]),Path(sys.argv[2]),sys.argv[3]
files={str(p.relative_to(source)):hashlib.sha256(p.read_bytes()).hexdigest() for p in source.rglob('*') if p.is_file() and not p.is_symlink()}
(stage/'receipt.json').write_text(json.dumps({'schema_version':1,'mode':'local_developer_id','identity':identity,'notarized':False,'published':False,'built_unsigned':files},ensure_ascii=False,indent=2)+'\n')
PY
codesign --force --options runtime --timestamp -s "$IDENTITY" "$APP/Contents/Resources/hydro-cli"
codesign --force --options runtime --timestamp -s "$IDENTITY" "$APP"
codesign --verify --deep --strict "$APP"
"$APP/Contents/Resources/bin/hydromac" --help >/dev/null
python3 - "$APP" "$STAGE" "$IDENTITY" <<'PY'
import hashlib,json,os,subprocess,sys
from pathlib import Path
app,stage,identity=Path(sys.argv[1]),Path(sys.argv[2]),sys.argv[3]
sig=subprocess.run(['codesign','-dvv',str(app)],capture_output=True,text=True,check=True)
text=sig.stdout+sig.stderr
assert 'Authority='+identity in text and 'runtime)' in text and '\nTimestamp=' in text
files={str(p.relative_to(app)):('link:'+os.readlink(p) if p.is_symlink() else hashlib.sha256(p.read_bytes()).hexdigest()) for p in app.rglob('*') if p.is_file() or p.is_symlink()}
(stage/'files.json').write_text(json.dumps(files,ensure_ascii=False,sort_keys=True,indent=2)+'\n')
(stage/'signature.txt').write_text(text)
PY
