#!/usr/bin/env python3
# 打完的 deb、rpm 去掉 WebKitGTK 的依赖（施工 AP-5）：Tauri 的打包工具（tauri-cli 2.12.4 `interface/rust.rs`）在 Linux 上不管配置写什么，
# 总给 deb 加 `libwebkit2gtk-4.1-0`、`libgtk-3-0`，给 rpm 加 `libwebkit2gtk-4.1.so.0()(64bit)`、`libgtk-3.so.0()(64bit)`，关不掉
# （AP-2 施工单「结果」）。托盘不用网页视图，装它的人不该被拉一份 WebKitGTK；GTK 3 照旧要（事件循环 tao、消息框 rfd），留着。
# 只改依赖那一项，程序和文件一个字节不动：
#   deb：ar 包里只换 control.tar.gz 里的 control（Depends 去掉 libwebkit2gtk*，重复的去掉），别的成员原样，成员头照抄只改大小。
#   rpm：主头里 REQUIRENAME/REQUIREFLAGS/REQUIREVERSION 三组去掉 libwebkit2gtk* 那几项，数据区照原来的顺序和对齐重排；签名头里
#        照主头算的 SHA1、SHA256 和「主头加内容」的大小换成新的（长度不变，原地改）。别的签名（GPG、MD5）认不出就停：第一版不签名。
# 没有 webkit 的照原样留着，只说一句；改完再读一遍，还有就算错。
#   python3 packaging/drop-webkit.py out/*.deb out/*.rpm
import gzip, hashlib, io, pathlib, struct, sys, tarfile

DROP = 'libwebkit2gtk'


def keep(name):
    return not name.strip().startswith(DROP)


# ---- deb ----

def ar_members(data):
    """ar 包的成员：(成员头 60 字节, 内容)。"""
    if not data.startswith(b'!<arch>\n'):
        sys.exit('不是 ar 包（deb）')
    at, members = 8, []
    while at < len(data):
        head = data[at:at + 60]
        size = int(head[48:58].decode().strip())
        members.append((head, data[at + 60:at + 60 + size]))
        at += 60 + size + (size & 1)
    return members


def ar_pack(members):
    out = bytearray(b'!<arch>\n')
    for head, body in members:
        out += head[:48] + str(len(body)).encode().ljust(10) + head[58:60] + body
        if len(body) & 1:
            out += b'\n'
    return bytes(out)


def depends_without_webkit(control):
    """control 的 Depends 一行：去掉 webkit 的、重复的。交回新的 control 和去掉的那几样。"""
    lines, dropped = control.split('\n'), []
    for i, line in enumerate(lines):
        if not line.startswith('Depends:'):
            continue
        kept = []
        for dep in line[len('Depends:'):].split(','):
            dep = dep.strip()
            if not keep(dep):
                dropped.append(dep)
            elif dep not in kept:
                kept.append(dep)
        lines[i] = 'Depends: ' + ', '.join(kept)
    return '\n'.join(lines), dropped


def fix_deb(path):
    members = ar_members(path.read_bytes())
    names = [head[:16].decode().strip().rstrip('/') for head, _ in members]
    if 'control.tar.gz' not in names:
        sys.exit(f'{path}：没有 control.tar.gz（成员 {names}），换了打包工具要重新看')
    index = names.index('control.tar.gz')
    head, body = members[index]
    entries = []
    with tarfile.open(fileobj=io.BytesIO(body), mode='r:gz') as tar:
        for info in tar.getmembers():
            entries.append((info, tar.extractfile(info).read() if info.isfile() else None))
    control = next((data for info, data in entries if info.name.lstrip('./') == 'control'), None)
    if control is None:
        sys.exit(f'{path}：control.tar.gz 里没有 control')
    text, dropped = depends_without_webkit(control.decode())
    if not dropped:
        print(f'{path}：Depends 里没有 {DROP}，不改')
        return
    raw = io.BytesIO()
    # 压缩的时间戳写 0：同样的输入出一样的字节
    with gzip.GzipFile(fileobj=raw, mode='wb', mtime=0) as gz, tarfile.open(fileobj=gz, mode='w', format=tarfile.GNU_FORMAT) as tar:
        for info, data in entries:
            if info.name.lstrip('./') == 'control':
                data = text.encode()
                info.size = len(data)
            tar.addfile(info, io.BytesIO(data) if data is not None else None)
    members[index] = (head, raw.getvalue())
    path.write_bytes(ar_pack(members))
    print(f'{path}：去掉 {", ".join(dropped)}；{next(l for l in text.split(chr(10)) if l.startswith("Depends:"))}')


# ---- rpm ----

# 签名头里照主头算的几样（rpm 的 rpmtag.h）：SHA1、SHA256 是主头的十六进制摘要，SIZE、LONGSIZE 是主头加内容的字节数。
SIG_SHA1, SIG_SHA256, SIG_SIZE, SIG_LONGSIZE = 269, 273, 1000, 270
# 认得、和主头无关的：头的区域标记（62）、留给以后签名的空位（1008）、rpm 6 的空位（999）。
SIG_OTHERS = {62, 999, 1008}
REQUIRE_FLAGS, REQUIRE_NAME, REQUIRE_VERSION = 1048, 1049, 1050
ALIGN = {3: 2, 4: 4, 5: 8}  # INT16、INT32、INT64 的对齐


def entry_size(store, typ, offset, count):
    """一项的数据在数据区里占多少字节。"""
    if typ in (0,):
        return 0
    if typ in (1, 2, 7):
        return count
    if typ in ALIGN:
        return ALIGN[typ] * count
    if typ in (6, 8, 9):  # STRING、STRING_ARRAY、I18NSTRING：count 个以 0 结尾的串
        at = offset
        for _ in range(count if typ != 6 else 1):
            at = store.index(b'\0', at) + 1
        return at - offset
    sys.exit(f'rpm 头里认不出的类型 {typ}')


def read_header(data, at):
    """一段头：[标签, 类型, 数量, 数据] 照数据区里的顺序，和它结束在哪。"""
    if data[at:at + 3] != b'\x8e\xad\xe8':
        sys.exit('rpm 头的开头不对')
    count, size = struct.unpack('>II', data[at + 8:at + 16])
    store_at = at + 16 + 16 * count
    store = data[store_at:store_at + size]
    entries = []
    for i in range(count):
        tag, typ, offset, n = struct.unpack('>iiii', data[at + 16 + 16 * i:at + 32 + 16 * i])
        entries.append([tag, typ, n, store[offset:offset + entry_size(store, typ, offset, n)], offset])
    return entries, store_at + size


def write_header(entries):
    """照原来的数据顺序重排数据区：同样的输入出原来的字节（rpm 自己也这样排）。"""
    store, index = bytearray(), {}
    for entry in sorted(entries, key=lambda e: e[4]):
        tag, typ, n, blob, _ = entry
        align = ALIGN.get(typ, 1)
        store += b'\0' * (-len(store) % align)
        index[id(entry)] = len(store)
        store += blob
    head = b'\x8e\xad\xe8\x01\0\0\0\0' + struct.pack('>II', len(entries), len(store))
    for entry in entries:
        head += struct.pack('>iiii', entry[0], entry[1], index[id(entry)], entry[2])
    return head + bytes(store)


def strings(blob):
    return blob[:-1].split(b'\0') if blob else []


def fix_rpm(path):
    data = path.read_bytes()
    if data[:4] != b'\xed\xab\xee\xdb':
        sys.exit(f'{path}：不是 rpm')
    sig, sig_end = read_header(data, 96)
    main_at = sig_end + (-sig_end % 8)
    main, main_end = read_header(data, main_at)
    if write_header(main) != data[main_at:main_end]:
        sys.exit(f'{path}：主头照原样重排出来对不上，不敢改')
    unknown = [e[0] for e in sig if e[0] not in SIG_OTHERS | {SIG_SHA1, SIG_SHA256, SIG_SIZE, SIG_LONGSIZE}]
    if unknown:
        sys.exit(f'{path}：签名头里有认不出的 {unknown}（签了名？），不敢改')
    by_tag = {e[0]: e for e in main}
    names = strings(by_tag[REQUIRE_NAME][3])
    drop = [i for i, name in enumerate(names) if not keep(name.decode())]
    if not drop:
        print(f'{path}：Requires 里没有 {DROP}，不改')
        return
    dropped = [names[i].decode() for i in drop]
    versions = strings(by_tag[REQUIRE_VERSION][3])
    flags = list(struct.unpack(f'>{len(names)}I', by_tag[REQUIRE_FLAGS][3]))
    if not (len(versions) == len(flags) == len(names)):
        sys.exit(f'{path}：Requires 三组的个数对不上')
    left = [i for i in range(len(names)) if i not in drop]
    by_tag[REQUIRE_NAME][2:4] = [len(left), b''.join(names[i] + b'\0' for i in left)]
    by_tag[REQUIRE_VERSION][2:4] = [len(left), b''.join(versions[i] + b'\0' for i in left)]
    by_tag[REQUIRE_FLAGS][2:4] = [len(left), struct.pack(f'>{len(left)}I', *(flags[i] for i in left))]
    header = write_header(main)
    payload = data[main_end:]
    # 签名头长度不变，原地换这几样
    sig_bytes = bytearray(data[96:sig_end])
    store_at = 16 + 16 * len(sig)
    fresh = {
        SIG_SHA1: hashlib.sha1(header).hexdigest().encode() + b'\0',
        SIG_SHA256: hashlib.sha256(header).hexdigest().encode() + b'\0',
        SIG_SIZE: struct.pack('>I', len(header) + len(payload)),
        SIG_LONGSIZE: struct.pack('>Q', len(header) + len(payload)),
    }
    for tag, _, _, blob, offset in sig:
        if tag in fresh:
            if len(fresh[tag]) != len(blob):
                sys.exit(f'{path}：签名头的 {tag} 长度变了')
            sig_bytes[store_at + offset:store_at + offset + len(blob)] = fresh[tag]
    path.write_bytes(data[:96] + bytes(sig_bytes) + data[sig_end:main_at] + header + payload)
    print(f'{path}：去掉 {", ".join(dropped)}')
    # 改完再读一遍
    again, _ = read_header(path.read_bytes(), main_at)
    left_names = strings(next(e for e in again if e[0] == REQUIRE_NAME)[3])
    if any(not keep(n.decode()) for n in left_names):
        sys.exit(f'{path}：改完还有 {DROP}')
    print(f'{path}：Requires {", ".join(n.decode() for n in left_names)}')


def main():
    if len(sys.argv) < 2:
        sys.exit('用法：python3 packaging/drop-webkit.py <deb、rpm>…')
    for name in sys.argv[1:]:
        path = pathlib.Path(name)
        if path.suffix == '.deb':
            fix_deb(path)
        elif path.suffix == '.rpm':
            fix_rpm(path)
        else:
            sys.exit(f'{path}：不是 deb、rpm')


if __name__ == '__main__':
    main()
