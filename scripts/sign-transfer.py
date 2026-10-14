#!/usr/bin/env python3
"""Transfer only this public bundle to the existing login-session signing runner."""
import argparse
import hashlib
import json
from pathlib import Path
import plistlib
import shlex
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
APP = ROOT / 'build/水利工具箱.app'
WORK = ROOT / 'build/install-product-basics-20261015'
IDENTITY = 'Developer ID Application: tianli Zeng (B9LJH93LA4)'
HELPER = Path.home() / 'Dev/tools/cc-home/skills/macmini/scripts/login_run.py'

def call(words, timeout=60):
    result = subprocess.run(list(map(str, words)), capture_output=True, text=True, timeout=timeout)
    if result.returncode:
        raise RuntimeError(f'{words[0]} exit {result.returncode}: {result.stdout[-2000:]} {result.stderr[-2000:]}')
    return result

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--host', default='macbook')
    args = parser.parse_args()
    def ssh(command): return call(['ssh', '-o', 'BatchMode=yes', args.host, command])
    WORK.mkdir(parents=True, exist_ok=True)
    info = plistlib.loads((APP / 'Contents/Info.plist').read_bytes())
    assert info['CFBundleIdentifier'] == 'io.github.zengtianli.HydroMac'
    build = str(info['CFBundleVersion'])
    name = 'hydrooss-basics-f0af7623-' + build
    peer_home = ssh('printf "%s" "$HOME"').stdout.strip()
    assert peer_home.startswith('/') and '\n' not in peer_home
    remote = peer_home + '/Library/Caches/mac-pair/login-run/' + name
    archive = WORK / ('input-' + build + '.zip')
    call(['/usr/bin/ditto', '-c', '-k', '--keepParent', APP, archive])
    signer = ROOT / 'scripts/sign-local.sh'
    manifest = WORK / ('input-' + build + '.sha256')
    manifest.write_text(sha(archive) + '  input.zip\n' + sha(signer) + '  sign-local.sh\n')
    state = ssh('launchctl print gui/$(id -u)/' + shlex.quote('cyou.tianli.oneoff.login-run-' + name) + ' >/dev/null 2>&1 && echo loaded || echo free')
    assert state.stdout.strip() == 'free', 'Signer already running; resume the original task'
    existing = ssh('if [ -f ' + shlex.quote(remote + '/signed.zip') + ' ]; then echo completed; else echo new; fi').stdout.strip()
    if existing == 'new':
        ssh('mkdir -p ' + shlex.quote(remote))
        for path, name_on_host in [(archive, 'input.zip'), (signer, 'sign-local.sh'), (manifest, 'input.sha256')]:
            call(['scp', '-q', path, args.host + ':' + shlex.quote(remote + '/' + name_on_host)])
        script = WORK / ('air-sign-' + build + '.sh')
        script.write_text('\n'.join([
            '#!/bin/zsh', 'set -euo pipefail', 'cd ' + shlex.quote(remote),
            '/usr/bin/shasum -a 256 -c input.sha256', 'mkdir -p unsigned',
            '/usr/bin/ditto -x -k input.zip unsigned',
            'CODE_SIGN_IDENTITY=' + shlex.quote(IDENTITY) + ' /bin/bash sign-local.sh ' + shlex.quote('unsigned/水利工具箱.app') + ' signed',
            '/usr/bin/ditto -c -k --keepParent signed signed.zip',
            '/usr/bin/shasum -a 256 signed.zip > signed.sha256', ''
        ]))
        print('Signing only through existing Air login_run: ' + name, flush=True)
        done = call([sys.executable, HELPER, '--host', args.host, '--name', name, '--script', script, '--timeout', '180', '--tail', '30'], timeout=240)
        (WORK / ('air-sign-' + build + '.log')).write_text(done.stdout + done.stderr)
        print(done.stdout, flush=True)
    else:
        original = ssh('cat ' + shlex.quote(remote + '/input.sha256')).stdout
        assert original == manifest.read_text(), 'Completed signing input changed; do not reuse'
    signed_zip, digest = WORK / ('signed-' + build + '.zip'), WORK / ('signed-' + build + '.sha256')
    for filename, target in [('signed.zip', signed_zip), ('signed.sha256', digest)]:
        call(['scp', '-q', args.host + ':' + shlex.quote(remote + '/' + filename), target])
    assert sha(signed_zip) == digest.read_text().split()[0], 'Signed transfer digest mismatch'
    extracted = WORK / ('returned-' + build)
    assert not extracted.exists(), 'Returned signing output already exists; read it before resuming'
    call(['/usr/bin/ditto', '-x', '-k', signed_zip, extracted])
    signed = extracted / 'signed'
    origin = json.loads((signed / 'receipt.json').read_text())
    assert all(sha(APP / p) == h for p, h in origin['built_unsigned'].items()), 'Signed input no longer equals the compiled public bundle'
    output = ROOT / 'build/accept/installed-source'
    assert not output.exists(), 'Output exists; do not overwrite provenance'
    call(['/usr/bin/ditto', signed, output])
    proof = {'schema_version': 1, 'identity': IDENTITY, 'host': args.host, 'remote_cache': remote,
             'input_archive_sha256': sha(archive), 'signed_archive_sha256': sha(signed_zip),
             'notarized': False, 'installed_on_signer': False, 'published': False}
    (WORK / 'signing-proof.json').write_text(json.dumps(proof, ensure_ascii=False, indent=2) + '\n')
    print('Signed public product returned: ' + str(output / '水利工具箱.app'), flush=True)

if __name__ == '__main__':
    main()
