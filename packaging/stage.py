#!/usr/bin/env python3
# 搭安装包的内容（施工 DT-6，图纸 app.md「装在哪」）：核心、网页软件编好以后，照 Tauri 打包工具要的样子放进 dist/。
#   dist/binaries/<程序>-<目标三元组>[.exe]  Tauri 的 externalBin：装在托盘程序旁边，名字去掉三元组
#   dist/resources/                         核心仓库的整份 resources/，加网页包的文件夹 packages/miyu-webui/
#   dist/resources/licenses/                托盘、核心、网页三份许可证和第三方许可证清单（施工 AP-5，原来外壳的 DT-7）
#   dist/pins.txt                           钉的两个提交号，写进发版说明
# 网页包的文件夹照网页仓库 tools/pack-web.py 装好的样子（网页的会话 2026-10-30 定）：清单没写 version 的补上（照网页的工作区版本）；
# 网页 W-1095（0.1.0）起清单自己写 version 和 targets 三种，照原样用，version 要和工作区版本一样，
# web.json、matugen/ 原样，许可证（仓库根的 LICENSE、miyu-webui/ 的第三方许可证清单）放包的根上，pages/ 是 web-demo 里给浏览器的七样，
# 程序放 bin/。等待页 loading.html 是给原来外壳的窗口用的，托盘不用，不带。
#   python3 packaging/stage.py --core <核心检出> --web <网页检出> --triple <rustc 的 host> --out dist
import argparse, pathlib, re, shutil, sys, tomllib

# 核心带的三个程序（核心的主会话 2026-10-30 定）：主程序、沙盒助手、抽字的小程序。
CORE_PROGRAMS = ['miyu', 'miyu-sandbox', 'miyu-extract']
WEB_PACKAGE = 'miyu-webui'
PAGES = ['index.html', 'distro.json', 'src', 'styles', 'packages', 'resources', 'vendor']


def workspace_version(root):
    text = (root / 'Cargo.toml').read_text(encoding='utf-8')
    found = re.search(r'^\[workspace\.package\]\n(?:.*\n)*?version\s*=\s*"([^"]+)"', text, re.M)
    if not found:
        sys.exit(f'{root}/Cargo.toml 的 [workspace.package] 里没有 version')
    return found.group(1)


def with_version(manifest, version):
    written = tomllib.loads(manifest).get('package', {}).get('version')
    if written is not None:
        # 网页 W-1095 起清单自己写：照原样，和工作区版本对不上的不装
        if written != version:
            sys.exit(f'{WEB_PACKAGE}/package.toml 的 version {written!r} 和网页的工作区版本 {version!r} 不一样：先找网页的会话')
        return manifest
    head = re.search(r'^\[package\]\n', manifest, re.M)
    if not head:
        sys.exit(f'{WEB_PACKAGE}/package.toml 里没有 [package]')
    return manifest[:head.end()] + f'version = "{version}"\n' + manifest[head.end():]


def copy_program(built, name, exe, dest):
    src = built / f'{name}{exe}'
    if not src.is_file():
        sys.exit(f'没编出来：{src}')
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dest)


LICENSES_README = """Licenses of the software in this Miyu package.

miyu-app/   the tray app (miyu-app): LICENSE (AGPL-3.0-or-later) and THIRD-PARTY-LICENSES.txt
miyu/       the core (miyu, miyu-sandbox, miyu-extract and these resources): LICENSE (AGPL-3.0-or-later),
            LICENSES/ (Apache-2.0 for the parts listed in the core's README) and THIRD-PARTY-LICENSES.txt
miyu-web/   the web app (packages/miyu-webui): LICENSE (AGPL-3.0-or-later) and THIRD-PARTY-LICENSES.txt
"""


def stage_licenses(app, core, web, dest):
    """三份许可证和第三方许可证清单放进资源目录的 licenses/（施工 AP-5，原来外壳的 DT-7）：缺一份就停，不出没有清单的包。
    核心、网页的清单是它们各自仓库生成、门禁守着的那一份，照钉的提交原样拷。"""
    if dest.exists():
        sys.exit(f'核心的资源目录里已经有 {dest.name}/：先问核心的会话')
    parts = {
        'miyu-app': (app, ['LICENSE', 'THIRD-PARTY-LICENSES.txt']),
        'miyu': (core, ['LICENSE', 'LICENSES', 'THIRD-PARTY-LICENSES.txt']),
        'miyu-web': (web, ['LICENSE', f'{WEB_PACKAGE}/THIRD-PARTY-LICENSES.txt']),
    }
    for part, (root, names) in parts.items():
        for name in names:
            src = root / name
            target = dest / part / pathlib.Path(name).name
            if src.is_dir():
                shutil.copytree(src, target)
            elif src.is_file():
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(src, target)
            else:
                sys.exit(f'少了 {src}：安装包要带它（施工 AP-5）')
    (dest / 'README.txt').write_text(LICENSES_README, encoding='utf-8')


def main():
    ap = argparse.ArgumentParser(description='搭 Miyu 安装包的内容')
    ap.add_argument('--core', required=True, type=pathlib.Path, help='核心的检出，已经编过 release')
    ap.add_argument('--web', required=True, type=pathlib.Path, help='网页的检出，已经编过 release')
    ap.add_argument('--triple', required=True, help='目标三元组，比如 x86_64-unknown-linux-gnu')
    ap.add_argument('--out', required=True, type=pathlib.Path)
    ap.add_argument('--core-built', type=pathlib.Path, help='核心编好的程序在哪，不写照 <核心检出>/target/release')
    ap.add_argument('--web-built', type=pathlib.Path, help='网页软件编好的程序在哪，不写照 <网页检出>/target/release')
    args = ap.parse_args()

    exe = '.exe' if 'windows' in args.triple else ''
    out = args.out.resolve()
    if out.exists():
        shutil.rmtree(out)

    for name in CORE_PROGRAMS:
        copy_program(args.core_built or args.core / 'target' / 'release', name, exe, out / 'binaries' / f'{name}-{args.triple}{exe}')

    resources = out / 'resources'
    shutil.copytree(args.core / 'resources', resources)
    package = resources / 'packages' / WEB_PACKAGE
    if package.exists():
        sys.exit(f'核心的资源目录里已经有 packages/{WEB_PACKAGE}/：先问核心的会话')
    source = args.web / WEB_PACKAGE
    package.mkdir(parents=True)
    manifest = (source / 'package.toml').read_text(encoding='utf-8')
    manifest = with_version(manifest, workspace_version(args.web))
    tomllib.loads(manifest)
    (package / 'package.toml').write_text(manifest, encoding='utf-8')
    shutil.copy2(source / 'web.json', package / 'web.json')
    shutil.copytree(source / 'matugen', package / 'matugen')
    # 许可证照 pack-web.py 放包的根上（网页的 0.1.0 起清单在 miyu-webui/ 里）：缺了就停，不出没有清单的包
    for src, name in ((source / 'THIRD-PARTY-LICENSES.txt', 'THIRD-PARTY-LICENSES.txt'), (args.web / 'LICENSE', 'LICENSE')):
        if not src.is_file():
            sys.exit(f'网页少了 {src}：安装包要带它（施工 AP-5）')
        shutil.copy2(src, package / name)
    for name in PAGES:
        page = args.web / 'web-demo' / name
        if page.is_dir():
            shutil.copytree(page, package / 'pages' / name)
        elif page.is_file():
            (package / 'pages').mkdir(exist_ok=True)
            shutil.copy2(page, package / 'pages' / name)
        else:
            sys.exit(f'网页少了 {page}')
    copy_program(args.web_built or args.web / 'target' / 'release', 'miyu-web', exe, package / 'bin' / f'miyu-web{exe}')

    app = pathlib.Path(__file__).resolve().parent.parent
    stage_licenses(app, args.core, args.web, resources / 'licenses')
    pins = tomllib.loads((app / 'pins.toml').read_text(encoding='utf-8'))
    (out / 'pins.txt').write_text(
        ''.join(f'{part}: {pins[part]["repo"]}@{pins[part]["rev"]}\n' for part in ('core', 'web')), encoding='utf-8')
    print(f'staged into {out}')


if __name__ == '__main__':
    main()
