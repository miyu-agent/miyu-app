#!/usr/bin/env python3
# 发版的版本号换成各种安装包认的写法（施工 AP-12）。版本号照核心设计 59「版本号」：年.月.这个月第几版（从 0 起、月不补零），
# 预发布加 -rc.N，例如 2026.11.0、2026.11.1、2026.11.0-rc.1；Cargo.toml 的 version 写的就是它。别的写法（现在的 0.1.0）照原样。
#   MSI：主版本、次版本最大 255，第三段最大 65535，预发布那段只能是纯数字（tauri-bundler msi/mod.rs 的 validate_wix_version），
#        2026 过不去。单给一个号（bundle.windows.wix.version）：年减 2000、月、第几版×100 加上 rc 号（正式版写 99）。
#        2026.11.0-rc.1 → 26.11.1，2026.11.0 → 26.11.99，2026.11.1 → 26.11.199。MSI 升级只比前三段，这样排得对；
#        rc 号最多 98，一个月最多 654 版，年份到 2255。
#   deb：- 后面是打包修订号，2026.11.0-rc.1 会排在 2026.11.0 后面，从 rc 升不到正式版。写成 2026.11.0~rc.1（~ 排在前面）。
#        Tauri 的配置只认 semver、写不了 ~：打的时候给 2026.11.0，打完照 drop-webkit.py 的拆法改 control 的 Version。
#   rpm：Tauri 用的 rpm 库不收带 - 的版本（rpm 0.30 util.rs validate_version），~ 同样写不进 Tauri 的配置。照 Fedora 原来的
#        预发布写法：版本 2026.11.0、发布号 0.rc.1（正式版的发布号是 1，0.rc.1 排在它前面）。
#   dmg、只装托盘的 tar.gz、zip：照原样。
#
#   python3 packaging/version.py --check                       换算的几个例子对不对
#   python3 packaging/version.py --set 2026.11.0-rc.1          试打：改 Cargo.toml、Cargo.lock、tauri.conf.json 的版本（不提交）
#   python3 packaging/version.py --tag v2026.11.0-rc.1         推标签时：正式标签要和 Cargo.toml 一样；演练标签照它改版本（同 --set）
#   python3 packaging/version.py --overlay windows --out F     打包时多叠的一份 Tauri 配置（用不着的是 {}）
#   python3 packaging/version.py --fix-deb out/*.deb           deb 的 Version 改成 ~ 的写法，文件名照 Cargo 的版本
#   python3 packaging/version.py --expect deb|rpm|msi          包里该是什么版本（核对用）
import argparse, importlib.util, io, gzip, json, pathlib, re, sys, tarfile, tomllib

ROOT = pathlib.Path(__file__).resolve().parent.parent
CALENDAR = re.compile(r'^(\d{4})\.(\d{1,2})\.(\d+)(?:-rc\.(\d+))?$')


def cargo_version():
    return tomllib.loads((ROOT / 'Cargo.toml').read_text(encoding='utf-8'))['package']['version']


def split(version):
    """(去掉预发布的版本, 预发布那段)；没有预发布的是 None。"""
    base, _, pre = version.partition('-')
    return base, pre or None


def wix(version):
    """MSI 的版本：年.月.第几版的换算；别的写法交 Tauri 自己换（None）。"""
    m = CALENDAR.match(version)
    if not m:
        return None
    year, month, n, rc = int(m[1]), int(m[2]), int(m[3]), m[4]
    if not (2000 <= year <= 2255 and 1 <= month <= 12):
        sys.exit(f'{version}：年要在 2000～2255、月要在 1～12')
    if rc is not None and not 1 <= int(rc) <= 98:
        sys.exit(f'{version}：rc 号要在 1～98（MSI 第三段的末两位，99 留给正式版）')
    build = n * 100 + (int(rc) if rc is not None else 99)
    if build > 65535:
        sys.exit(f'{version}：一个月最多 654 版（MSI 第三段最大 65535）')
    return f'{year - 2000}.{month}.{build}'


def deb(version):
    return version.replace('-', '~')


def rpm(version):
    """(Version, Release)。"""
    base, pre = split(version)
    return (base, f'0.{pre.replace("-", ".")}') if pre else (version, '1')


def overlay(platform, version):
    if platform == 'windows':
        w = wix(version)
        return {'bundle': {'windows': {'wix': {'version': w}}}} if w else {}
    if platform == 'linux':
        base, pre = split(version)
        return {'version': base, 'bundle': {'linux': {'rpm': {'release': rpm(version)[1]}}}} if pre else {}
    return {}


def check():
    cases = {
        '2026.11.0-rc.1': ('26.11.1', '2026.11.0~rc.1', ('2026.11.0', '0.rc.1')),
        '2026.11.0': ('26.11.99', '2026.11.0', ('2026.11.0', '1')),
        '2026.11.1': ('26.11.199', '2026.11.1', ('2026.11.1', '1')),
        '2026.12.0-rc.2': ('26.12.2', '2026.12.0~rc.2', ('2026.12.0', '0.rc.2')),
        '0.1.0': (None, '0.1.0', ('0.1.0', '1')),
    }
    for version, want in cases.items():
        got = (wix(version), deb(version), rpm(version))
        if got != want:
            sys.exit(f'{version}：换出来是 {got}，该是 {want}')
    # 排得对：MSI 照前三段的数比
    order = ['2026.11.0-rc.1', '2026.11.0-rc.2', '2026.11.0', '2026.11.1-rc.1', '2026.11.1', '2026.12.0-rc.1', '2027.1.0']
    keys = [tuple(int(x) for x in wix(v).split('.')) for v in order]
    if keys != sorted(keys) or len(set(keys)) != len(keys):
        sys.exit(f'MSI 的号排得不对：{list(zip(order, keys))}')
    print('换算的例子都对')


def set_version(version):
    """试打用：三处的版本换成 `version`，不提交。"""
    old = cargo_version()
    cargo = ROOT / 'Cargo.toml'
    text = cargo.read_text(encoding='utf-8')
    head = re.search(r'^\[package\]\n(?:.*\n)*?version = "[^"]*"', text, re.M)
    text = text[:head.start()] + re.sub(r'version = "[^"]*"$', f'version = "{version}"', head.group(0)) + text[head.end():]
    cargo.write_text(text, encoding='utf-8', newline='\n')
    lock = ROOT / 'Cargo.lock'
    text, n = re.subn(rf'(\[\[package\]\]\nname = "miyu-applet"\nversion = )"{re.escape(old)}"', rf'\1"{version}"', lock.read_text(encoding='utf-8'))
    if n != 1:
        sys.exit('Cargo.lock 里找不到 miyu-applet 那一项')
    lock.write_text(text, encoding='utf-8', newline='\n')
    conf = ROOT / 'tauri.conf.json'
    text, n = re.subn(r'("version": )"[^"]*"', rf'\1"{version}"', conf.read_text(encoding='utf-8'), count=1)
    if n != 1:
        sys.exit('tauri.conf.json 里没有 version')
    conf.write_text(text, encoding='utf-8', newline='\n')
    print(f'版本 {old} → {version}（Cargo.toml、Cargo.lock、tauri.conf.json）')


def follow_tag(tag):
    """推标签打包时（施工 AP-13，照核心 RL-2补2）：正式标签 v<版本> 要和 Cargo.toml 的一样，什么都不动；演练标签
    v<年.月.序号>-rc.N 只在这台机器上把版本改成标签里的（不往 main 提交 rc 号）。Cargo.toml 已经是年.月.序号的，演练标签的
    前半段也要和它一样；还是别的写法（现在的 0.1.0，等发版那天和核心一起定号）的不比。"""
    version = cargo_version()
    if tag == f'v{version}':
        print(f'正式标签 {tag}：版本照 Cargo.toml 的 {version}')
        return
    m = re.match(r'^v((\d{4})\.(\d{1,2})\.(\d+))-rc\.(\d+)$', tag)
    if not m or not CALENDAR.match(tag[1:]):
        sys.exit(f'标签 {tag} 不对：要 v{version}（正式），或者 v<年.月.序号>-rc.N（演练）')
    if CALENDAR.match(version) and split(version)[1] is None and m[1] != version:
        sys.exit(f'演练标签 {tag} 和 Cargo.toml 的版本 {version} 对不上（要 v{version}-rc.N）')
    wix(tag[1:])  # 年、月、rc 号的范围照 MSI 那一套查，不对的停下
    print(f'演练标签 {tag}：版本临时改成 {tag[1:]}（只在这台机器上，不提交）')
    set_version(tag[1:])


def fix_deb(paths, version):
    """control 的 Version 换成 ~ 的写法；Tauri 照去掉预发布的版本起的文件名换成照 Cargo 的版本。"""
    want = deb(version)
    spec = importlib.util.spec_from_file_location('drop_webkit', ROOT / 'packaging' / 'drop-webkit.py')
    ar = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(ar)
    for path in map(pathlib.Path, paths):
        members = ar.ar_members(path.read_bytes())
        names = [head[:16].decode().strip().rstrip('/') for head, _ in members]
        index = names.index('control.tar.gz')
        head, body = members[index]
        entries = []
        with tarfile.open(fileobj=io.BytesIO(body), mode='r:gz') as tar:
            for info in tar.getmembers():
                entries.append((info, tar.extractfile(info).read() if info.isfile() else None))
        control = next(data for info, data in entries if info.name.lstrip('./') == 'control').decode()
        got = re.search(r'^Version: (.*)$', control, re.M)[1]
        if got != want:
            control = re.sub(r'^Version: .*$', f'Version: {want}', control, count=1, flags=re.M)
            raw = io.BytesIO()
            with gzip.GzipFile(fileobj=raw, mode='wb', mtime=0) as gz, tarfile.open(fileobj=gz, mode='w', format=tarfile.GNU_FORMAT) as tar:
                for info, data in entries:
                    if info.name.lstrip('./') == 'control':
                        data = control.encode()
                        info.size = len(data)
                    tar.addfile(info, io.BytesIO(data) if data is not None else None)
            members[index] = (head, raw.getvalue())
            path.write_bytes(ar.ar_pack(members))
            print(f'{path}：Version {got} → {want}')
        else:
            print(f'{path}：Version 已经是 {want}')
        base = split(version)[0]
        if base != version and f'_{base}_' in path.name:
            moved = path.with_name(path.name.replace(f'_{base}_', f'_{version}_'))
            path.rename(moved)
            print(f'{path.name} → {moved.name}')


def main():
    # Windows 的控制台默认 cp1252，中文打不出来（2026-11-06 打包流水线在 Windows 上挂在这里）
    for stream in (sys.stdout, sys.stderr):
        stream.reconfigure(encoding='utf-8')
    ap = argparse.ArgumentParser(description='发版的版本号换成各种安装包认的写法')
    ap.add_argument('--check', action='store_true', help='换算的几个例子对不对')
    ap.add_argument('--set', metavar='版本', help='试打：改 Cargo.toml、Cargo.lock、tauri.conf.json 的版本')
    ap.add_argument('--tag', metavar='标签', help='推标签时：正式标签核对版本，演练标签照它改版本')
    ap.add_argument('--overlay', choices=['linux', 'macos', 'windows'], help='打包时多叠的一份 Tauri 配置')
    ap.add_argument('--out', help='--overlay 写到哪')
    ap.add_argument('--fix-deb', nargs='+', metavar='deb', help='deb 的 Version 改成 ~ 的写法')
    ap.add_argument('--expect', choices=['deb', 'rpm', 'msi'], help='包里该是什么版本')
    args = ap.parse_args()
    if args.check:
        check()
    elif args.set:
        set_version(args.set)
    elif args.tag:
        follow_tag(args.tag)
    elif args.overlay:
        if not args.out:
            sys.exit('--overlay 要带 --out')
        data = overlay(args.overlay, cargo_version())
        pathlib.Path(args.out).write_text(json.dumps(data, ensure_ascii=False) + '\n', encoding='utf-8', newline='\n')
        print(f'{args.out}：{json.dumps(data, ensure_ascii=False)}')
    elif args.fix_deb:
        fix_deb(args.fix_deb, cargo_version())
    elif args.expect:
        v = cargo_version()
        print({'deb': deb(v), 'rpm': '-'.join(rpm(v)), 'msi': wix(v) or v}[args.expect])
    else:
        ap.print_help()


if __name__ == '__main__':
    main()
