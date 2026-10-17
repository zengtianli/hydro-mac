#!/usr/bin/env python3
"""Build and optionally install the current product locally, without publication."""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
APP_NAME = '水利工具箱.app'
PATTERNS = ['Sources/**', 'build.sh', 'scripts/*.py', 'scripts/*.sh', 'scripts/hydromac',
            'cli/src/**', 'cli/data/**', 'cli/Cargo.toml', 'cli/Cargo.lock', 'icon/**']


def sign_built(stage):
    """Separate producer phase, executed inside Chapter's real build transaction."""
    stage = stage.resolve()
    if stage.parent != ROOT / 'build' or not stage.name.startswith('local-fast-'):
        raise ValueError('Signing output must be this product local-fast build directory')
    target = stage / APP_NAME
    subprocess.run(['ditto', str(ROOT / 'build' / APP_NAME), str(target)], check=True)
    sys.path.insert(0, str(Path.home() / 'Dev/tools/dev/lib/tools/macapp'))
    import distribution_grade
    facts = distribution_grade.sign(target, identity=os.environ.get('CODE_SIGN_IDENTITY'))
    (stage / 'signature.json').write_text(json.dumps(facts, ensure_ascii=False, indent=2) + '\n')
    manifest = {str(p.relative_to(target)): ('link:' + os.readlink(p) if p.is_symlink()
                else hashlib.sha256(p.read_bytes()).hexdigest())
                for p in target.rglob('*') if p.is_file() or p.is_symlink()}
    (stage / 'files.json').write_text(json.dumps(manifest, ensure_ascii=False, sort_keys=True, indent=2) + '\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--install', action='store_true', help='Use the existing data-preserving installer after building')
    parser.add_argument('--sign-built', type=Path, help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.sign_built:
        sign_built(args.sign_built)
        return 0
    (ROOT / 'build').mkdir(exist_ok=True)
    # Same product-output guard as test_product.py; other products retain their SDK slots.
    with (ROOT / 'build/current-source-build.lock').open('a') as guard:
        try:
            fcntl.flock(guard, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            print('Build deferred: this product build/test entry is already running', file=sys.stderr)
            return 75
        sys.path.insert(0, str(Path.home() / 'Dev/tools/dev/lib/tools/macapp/ios'))
        import sim_lane
        try:
            with sim_lane.BuildSlot('hydro-mac-oss local fast build', wait=0):
                sys.path.insert(0, str(Path.home() / 'Apps/chapter/engine'))
                import app_sop
                product = app_sop.load_apps('hydro-mac-oss')[0]
                if Path(product['repo']).resolve() != ROOT:
                    raise ValueError('Chapter registration must point to this product repository')
                stage = Path(tempfile.mkdtemp(prefix='local-fast-', dir=ROOT / 'build'))
                receipt = ROOT / 'perf/build-receipt.json'
                if receipt.exists():
                    (stage / 'before-build-receipt.json').write_bytes(receipt.read_bytes())
                command = 'bash build.sh && ' + shlex.join([
                    sys.executable, str(Path(__file__).resolve()), '--sign-built', str(stage)])
                artifact = stage / APP_NAME
                app_sop.build_receipt(product, artifact, PATTERNS, command, output_path=receipt)
                valid, reason = app_sop.verify_build_receipt(product, artifact, receipt_path=receipt)
                if not valid:
                    raise ValueError(reason)
                (stage / 'build-receipt.json').write_bytes(receipt.read_bytes())
        except sim_lane.Busy as error:
            print('Build deferred: ' + str(error), file=sys.stderr)
            return 75
        except app_sop.BuildCommandError as error:
            print(str(error), file=sys.stderr)
            return error.returncode
        # Installation is outside the SDK slot, retaining the product-output guard.
        if args.install:
            subprocess.run([sys.executable, str(ROOT / 'scripts/install-local.py'),
                            '--app', str(artifact), '--receipt', str(receipt)], check=True, cwd=ROOT)
        print(json.dumps({'artifact': str(artifact), 'receipt': str(receipt),
                          'installed': args.install, 'notarization_requested': False,
                          'published': False}, ensure_ascii=False), flush=True)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
