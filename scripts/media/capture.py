#!/usr/bin/env python3
"""Record the installed product's real mounted rainfall actions without any system input."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import uuid

ROOT = Path(__file__).resolve().parents[2]
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--app', type=Path, default=Path('/Applications/水利工具箱.app'))
parser.add_argument('--receipt', type=Path, default=ROOT / 'perf/build-receipt.json')
parser.add_argument('--out', type=Path, required=True)
args = parser.parse_args()
out = args.out.resolve()
assert out.is_relative_to(ROOT / 'build') and not out.exists(), 'Use a fresh product build media directory'
receipt = json.loads(args.receipt.read_text()); artifact = receipt['artifact']
info = plistlib.loads((args.app / 'Contents/Info.plist').read_bytes())
assert info['CFBundleIdentifier'] == artifact['bundle_id'] == 'io.github.zengtianli.HydroMac'
assert info['CFBundleVersion'] == artifact['build'] and info['CFBundleShortVersionString'] == artifact['version']
assert sha(args.app / artifact['executable']) == artifact['sha256']
assert all(sha(ROOT / p) == h for p, h in receipt['source']['files'].items())
subprocess.run(['codesign', '--verify', '--deep', '--strict', str(args.app)], check=True)
out.mkdir(parents=True)
domain = 'test.hydromac.ui.' + uuid.uuid4().hex
clone = out / 'Product.app'; shutil.copytree(args.app, clone, symlinks=True)
info.update(CFBundleIdentifier=domain, LSUIElement=True)
(clone / 'Contents/Info.plist').write_bytes(plistlib.dumps(info))
subprocess.run(['codesign', '--force', '--deep', '--sign', '-', str(clone)], check=True, capture_output=True)
env = {k: v for k, v in os.environ.items() if k not in ['AMAP_KEY', 'AMAP_API_KEY', 'HYDRO_CLI_PATH'] and not k.startswith(('APP_LIFECYCLE_', 'HYDROMAC_'))}
env.update(HYDROMAC_UI_OUT=str(out / 'ui'), HYDROMAC_UI_RECORD='1', APP_LIFECYCLE_SUPPORT_DIR=str(out / 'support'), APP_LIFECYCLE_CLOUD_DIR=str(out / 'cloud'))
try:
    done = subprocess.run([str(clone / artifact['executable']), '--ui-self-test'], env=env, capture_output=True, text=True, timeout=90)
    (out / 'capture.log').write_text(done.stdout + done.stderr)
    assert done.returncode == 0, (done.stdout + done.stderr)[-2000:]
finally:
    subprocess.run(['defaults', 'delete', domain], capture_output=True)
ui = out / 'ui'; raw = out / 'raw.mp4'
capture = json.loads((ui / 'capture.json').read_text())
assert len(capture['frames']) > 100
subprocess.run(['ffmpeg', '-v', 'error', '-framerate', '15', '-i', str(ui / 'frames/%05d.png'), '-c:v', 'libx264', '-pix_fmt', 'yuv420p', '-crf', '18', '-movflags', '+faststart', str(raw)], check=True)
capture.update(source_sha256=sha(raw), app_id='hydro-mac-oss', version=artifact['version'], build=artifact['build'],
               artifact_sha256=artifact['sha256'], source_receipt_sha256=sha(args.receipt), source_files_sha256=receipt['source']['sha256'])
native = out / 'native-capture.json'; native.write_text(json.dumps(capture, ensure_ascii=False, indent=2) + '\n')
events = out / 'events.jsonl'
events.write_text('\n'.join(json.dumps(e, ensure_ascii=False) for e in [{'type': 'header', 'display': {'w': 1180, 'h': 760}}, *capture['navigation_events']]) + '\n')
duration = capture['duration']; e = {x['action']: x['t'] for x in capture['navigation_events']}
plan = {'schema': 1, 'title': '水利工具箱：降雨管线', 'source': str(raw), 'source_size': [1180, 760],
        'capture_kind': 'continuous', 'capture_backend': 'native-appkit-surface', 'cursor_mode': 'recorded', 'sync_marker': False,
        'events': str(events), 'events_time': 'clip', 'duration': duration, 'native_capture': {'path': str(native), 'sha256': sha(native)},
        'version': artifact['version'], 'build': artifact['build'],
        'actions': [{'label': '加载合成示例', 't': e['load_sample_started'], 'bbox': [225, 18, 130, 30]},
                    {'label': '运行真实六步管线', 't': e['run_pipeline_started'], 'bbox': [225, 78, 95, 30]},
                    {'label': '查看分区汇总', 't': e['partition_tab_selected'], 'bbox': [350, 134, 85, 30]},
                    {'label': '查看完整结果预览', 't': e['data_tab_selected'], 'bbox': [510, 134, 85, 30]}],
        'zooms': [{'start': .4, 'end': e['run_pipeline_completed'] + .6, 'scale': 1.25, 'center': [455, 140]},
                  {'start': e['partition_tab_selected'] + .3, 'end': duration - .3, 'scale': 1.25, 'center': [580, 380]}],
        'narration': [{'start': .1, 'text': '加载合成示例。'}, {'start': e['run_pipeline_started'] + .1, 'text': '运行六步管线。'},
                      {'start': e['partition_tab_selected'] + .1, 'text': '查看分区汇总。'}, {'start': e['data_tab_selected'] + .1, 'text': '查看结果预览。'}],
        'chapters': [{'start': 0, 'title': '加载示例'}, {'start': e['run_pipeline_started'], 'title': '运行管线'},
                     {'start': e['partition_tab_selected'], 'title': '查看结果'}],
        'output': {'width': 1180, 'height': 760, 'fps': 30, 'ramp': .85, 'padding': 0}}
(out / 'plan.json').write_text(json.dumps(plan, ensure_ascii=False, indent=2) + '\n')
print(json.dumps({'plan': str(out / 'plan.json'), 'frames': len(capture['frames']), 'duration': duration, 'raw_sha256': sha(raw)}))
