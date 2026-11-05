#!/usr/bin/env python3
# 只装托盘的包（施工 AP-11，核心设计 60：AUR、Homebrew 里托盘单独成包 miyu-applet，依赖 miyu-core、miyu-webui）：不带核心、网页，
# 托盘照 PATH 找系统装的 miyu（src/program.rs）。和完整的 deb、rpm、msi、dmg（设计 59，带核心和网页、装上就能用）分开出。
#   Linux：miyu-applet-<版本>-linux-<架构>.tar.gz，里面一层 miyu-applet-<版本>-linux-<架构>/，照 /usr 的样子摆：
#     bin/miyu-applet
#     share/applications/miyu-applet.desktop   照 packaging/linux/miyu-applet.desktop 填（同 deb、rpm 的那一份）
#     share/icons/hicolor/<尺寸>/apps/miyu-applet.png
#     share/licenses/miyu-applet/LICENSE、THIRD-PARTY-LICENSES.txt
#   macOS：只搭 --licenses 那个文件夹（托盘自己的两份），.app 由 Tauri 的打包工具照 packaging/macos-solo.json 打，release.yml 用 ditto 压成
#     miyu-applet-<版本>-macos-aarch64.zip。
#   python3 packaging/solo.py --binary target/release/miyu-applet --target linux-x86_64 --out out
#   python3 packaging/solo.py --licenses dist-solo/licenses
import argparse, io, pathlib, re, tarfile, tomllib

ROOT = pathlib.Path(__file__).resolve().parent.parent
NAME = 'miyu-applet'
TARGETS = ('linux-x86_64', 'linux-aarch64')
ICONS = ('32x32', '128x128', '256x256')
LICENSES = ('LICENSE', 'THIRD-PARTY-LICENSES.txt')


def version():
    return tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']


def desktop():
    """packaging/linux/ 的模板照 Tauri 打 deb 时的填法填（{{exec}}、{{icon}}、{{categories}}）。"""
    text = (ROOT / 'packaging' / 'linux' / f'{NAME}.desktop').read_text()
    text = text.replace('{{exec}}', NAME).replace('{{icon}}', NAME).replace('{{categories}}', 'Utility;')
    left = re.findall(r'\{\{[^}]*\}\}', text)
    if left:
        raise SystemExit(f'{NAME}.desktop 里还有没填的：{left}')
    return text


def licenses(out):
    out = pathlib.Path(out)
    out.mkdir(parents=True, exist_ok=True)
    for f in LICENSES:
        (out / f).write_bytes((ROOT / f).read_bytes())


def tarball(binary, target, out):
    if target not in TARGETS:
        raise SystemExit(f'--target 只认 {", ".join(TARGETS)}')
    binary = pathlib.Path(binary)
    if not binary.is_file():
        raise SystemExit(f'没有 {binary}')
    top = f'{NAME}-{version()}-{target}'
    files = [(f'bin/{NAME}', binary.read_bytes(), 0o755), (f'share/applications/{NAME}.desktop', desktop().encode(), 0o644)]
    files += [(f'share/icons/hicolor/{size}/apps/{NAME}.png', (ROOT / 'icons' / f'{size}.png').read_bytes(), 0o644) for size in ICONS]
    files += [(f'share/licenses/{NAME}/{f}', (ROOT / f).read_bytes(), 0o644) for f in LICENSES]
    out = pathlib.Path(out)
    out.mkdir(parents=True, exist_ok=True)
    path = out / f'{top}.tar.gz'
    with tarfile.open(path, 'w:gz') as tar:
        dirs = sorted({'/'.join(name.split('/')[:i]) for name, _, _ in files for i in range(1, name.count('/') + 1)})
        for d in [''] + dirs:
            info = tarfile.TarInfo(f'{top}/{d}'.rstrip('/'))
            info.type, info.mode = tarfile.DIRTYPE, 0o755
            tar.addfile(info)
        for name, data, mode in files:
            info = tarfile.TarInfo(f'{top}/{name}')
            info.size, info.mode = len(data), mode
            tar.addfile(info, io.BytesIO(data))
    print(path)


def main():
    ap = argparse.ArgumentParser(description='只装托盘的包（施工 AP-11）')
    ap.add_argument('--binary', help='编好的 miyu-applet')
    ap.add_argument('--target', help='linux-x86_64 或 linux-aarch64')
    ap.add_argument('--out', help='tar.gz 放哪个目录')
    ap.add_argument('--licenses', help='只搭许可证的文件夹（macOS 打 .app 用）')
    a = ap.parse_args()
    if a.licenses:
        licenses(a.licenses)
    elif a.binary and a.target and a.out:
        tarball(a.binary, a.target, a.out)
    else:
        ap.error('要 --licenses，或者 --binary、--target、--out 一起')


if __name__ == '__main__':
    main()
